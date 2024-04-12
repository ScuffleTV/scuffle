use std::sync::Arc;

use futures_util::{Stream, StreamExt};
use postgres_from_row::FromRow;
use postgres_types::{FromSql, ToSql};
use tokio_postgres::{Error, Row};

pub fn query<'a>(query: impl ToString) -> QueryBuilder<'a> {
	QueryBuilder::new(query)
}

#[derive(Default)]
pub struct QueryBuilder<'a> {
	query: String,
	params: Vec<Box<dyn ToSql + Send + Sync + 'a>>,
}

impl<'args> QueryBuilder<'args> {
	pub fn new(query: impl ToString) -> Self {
		Self {
			query: query.to_string(),
			params: Vec::new(),
		}
	}

	pub fn push_bind(&mut self, param: impl ToSql + Send + Sync + 'args) -> &mut Self {
		self.params.push(Box::new(param));
		self.query.push_str(format!("${}", self.params.len()).as_str());
		self
	}

	pub fn bind(&mut self, param: impl ToSql + Send + Sync + 'args) -> &mut Self {
		self.params.push(Box::new(param));
		self
	}

	pub fn push(&mut self, query: impl AsRef<str>) -> &mut Self {
		self.query.push_str(query.as_ref());
		self
	}

	pub fn separated(&mut self, sep: &'args str) -> Separated<'_, 'args> {
		Separated {
			sep,
			first: true,
			query_builder: self,
		}
	}

	pub fn push_tuples<T>(
		&mut self,
		tuples: impl IntoIterator<Item = T>,
		mut f: impl FnMut(Separated<'_, 'args>, T),
	) -> &mut Self {
		self.push(" (");

		let mut separated = self.separated(",");

		for tuple in tuples {
			separated.push("(");

			f(separated.query_builder.separated(", "), tuple);

			separated.push_unseparated(")");
		}

		separated.push_unseparated(")");

		separated.query_builder
	}

	pub fn push_values<T>(
		&mut self,
		values: impl IntoIterator<Item = T>,
		mut f: impl FnMut(Separated<'_, 'args>, T),
	) -> &mut Self {
		self.push("VALUES ");

		let mut separated = self.separated(",");

		for value in values {
			separated.push("(");

			f(separated.query_builder.separated(", "), value);

			separated.push_unseparated(")");
		}

		separated.query_builder
	}

	pub fn build(&self) -> Query<'_, NoParse, Row> {
		Query {
			query: &self.query,
			params: &self.params,
			_marker: std::marker::PhantomData,
		}
	}

	pub fn build_query_as<T: FromRow>(&self) -> Query<'_, FromRowParse<T>, T> {
		Query {
			query: &self.query,
			params: &self.params,
			_marker: std::marker::PhantomData,
		}
	}

	pub fn build_query_scalar<T: SqlScalar>(&self) -> Query<'_, ScalarParse<T>, T> {
		Query {
			query: &self.query,
			params: &self.params,
			_marker: std::marker::PhantomData,
		}
	}

	pub fn build_query_single_scalar<T: for<'a> FromSql<'a>>(&self) -> Query<'_, SingleScalarParse<T>, T> {
		Query {
			query: &self.query,
			params: &self.params,
			_marker: std::marker::PhantomData,
		}
	}

	pub fn sql(&self) -> &str {
		self.query.as_str()
	}
}

pub struct ScalarParse<T>(std::marker::PhantomData<T>);

pub struct SingleScalarParse<T>(std::marker::PhantomData<T>);

pub struct FromRowParse<T>(std::marker::PhantomData<T>);
pub struct NoParse;

impl<T: SqlScalar> RowParse for ScalarParse<T> {
	type Item = T;

	#[inline]
	fn try_from_row(row: Row) -> Result<Self::Item, Error> {
		T::from_row(&row)
	}
}

impl<T> RowParse for SingleScalarParse<T>
where
	T: for<'a> FromSql<'a>,
{
	type Item = T;

	#[inline]
	fn try_from_row(row: Row) -> Result<Self::Item, Error> {
		row.try_get(0)
	}
}

impl<T: FromRow> RowParse for FromRowParse<T> {
	type Item = T;

	#[inline]
	fn try_from_row(row: Row) -> Result<Self::Item, Error> {
		T::try_from_row(&row)
	}
}

impl RowParse for NoParse {
	type Item = Row;

	#[inline]
	fn try_from_row(row: Row) -> Result<Self::Item, Error> {
		Ok(row)
	}
}

pub trait RowParse {
	type Item;

	fn try_from_row(row: Row) -> Result<Self::Item, Error>;
}

pub struct Query<'a, T: RowParse<Item = O>, O> {
	query: &'a str,
	params: &'a [Box<dyn ToSql + Send + Sync + 'a>],
	_marker: std::marker::PhantomData<(T, O)>,
}

fn params<'a>(params: &'a [Box<dyn ToSql + Send + Sync + 'a>]) -> Vec<&'a (dyn ToSql + Sync)> {
	params.iter().map(|param| param.as_ref() as _).collect()
}

pub enum Client<'a, C> {
	Owned(C),
	Borrowed(&'a C),
}

impl<C: AsRef<tokio_postgres::Client>> std::ops::Deref for Client<'_, C> {
	type Target = tokio_postgres::Client;

	fn deref(&self) -> &Self::Target {
		match self {
			Client::Owned(client) => client.as_ref(),
			Client::Borrowed(client) => client.as_ref(),
		}
	}
}

/// A trait that represents a client-like object that can be used to build
/// queries.
pub trait ClientLike: Send + Sync {
	#[doc(hidden)]
	fn query_builder_client(
		&self,
	) -> impl std::future::Future<Output = Result<impl AsRef<tokio_postgres::Client> + '_, deadpool_postgres::PoolError>> + Send;
}

struct ClientWrapper<'a>(&'a tokio_postgres::Client);

impl AsRef<tokio_postgres::Client> for ClientWrapper<'_> {
	fn as_ref(&self) -> &tokio_postgres::Client {
		self.0
	}
}

struct TransactionWrapper<'a>(&'a tokio_postgres::Transaction<'a>);

impl AsRef<tokio_postgres::Client> for TransactionWrapper<'_> {
	fn as_ref(&self) -> &tokio_postgres::Client {
		self.0.client()
	}
}

struct PoolClientWrapperOwned(deadpool_postgres::Client);

impl AsRef<tokio_postgres::Client> for PoolClientWrapperOwned {
	fn as_ref(&self) -> &tokio_postgres::Client {
		self.0.as_ref()
	}
}

struct PoolClientWrapperBorrowed<'a>(&'a deadpool_postgres::Client);

impl AsRef<tokio_postgres::Client> for PoolClientWrapperBorrowed<'_> {
	fn as_ref(&self) -> &tokio_postgres::Client {
		self.0.as_ref()
	}
}

struct PoolTransactionWrapper<'a>(&'a deadpool_postgres::Transaction<'a>);

impl AsRef<tokio_postgres::Client> for PoolTransactionWrapper<'_> {
	fn as_ref(&self) -> &tokio_postgres::Client {
		self.0.client()
	}
}

impl ClientLike for tokio_postgres::Client {
	async fn query_builder_client(&self) -> Result<impl AsRef<tokio_postgres::Client> + '_, deadpool_postgres::PoolError> {
		Ok(ClientWrapper(self))
	}
}

impl ClientLike for tokio_postgres::Transaction<'_> {
	async fn query_builder_client(&self) -> Result<impl AsRef<tokio_postgres::Client> + '_, deadpool_postgres::PoolError> {
		Ok(TransactionWrapper(self))
	}
}

impl ClientLike for deadpool_postgres::Pool {
	async fn query_builder_client(&self) -> Result<impl AsRef<tokio_postgres::Client> + '_, deadpool_postgres::PoolError> {
		Ok(PoolClientWrapperOwned(self.get().await?))
	}
}

impl ClientLike for deadpool_postgres::Client {
	async fn query_builder_client(&self) -> Result<impl AsRef<tokio_postgres::Client> + '_, deadpool_postgres::PoolError> {
		Ok(PoolClientWrapperBorrowed(self))
	}
}

impl ClientLike for deadpool_postgres::Transaction<'_> {
	async fn query_builder_client(&self) -> Result<impl AsRef<tokio_postgres::Client> + '_, deadpool_postgres::PoolError> {
		Ok(PoolTransactionWrapper(self))
	}
}

impl<T: ClientLike + Sync + Send> ClientLike for Arc<T> {
	async fn query_builder_client(&self) -> Result<impl AsRef<tokio_postgres::Client> + '_, deadpool_postgres::PoolError> {
		self.as_ref().query_builder_client().await
	}
}

impl<T: ClientLike + Sync> ClientLike for &T {
	async fn query_builder_client(&self) -> Result<impl AsRef<tokio_postgres::Client> + '_, deadpool_postgres::PoolError> {
		(*self).query_builder_client().await
	}
}

impl<T: RowParse<Item = O>, O> Query<'_, T, O> {
	pub async fn execute(self, conn: impl ClientLike) -> Result<u64, deadpool_postgres::PoolError> {
		Ok(conn
			.query_builder_client()
			.await?
			.as_ref()
			.execute(self.query, &params(self.params))
			.await?)
	}

	pub async fn fetch_all(self, conn: impl ClientLike) -> Result<Vec<O>, deadpool_postgres::PoolError> {
		Ok(conn
			.query_builder_client()
			.await?
			.as_ref()
			.query(self.query, &params(self.params))
			.await?
			.into_iter()
			.map(T::try_from_row)
			.collect::<Result<_, Error>>()?)
	}

	pub async fn fetch_one(self, conn: impl ClientLike) -> Result<O, deadpool_postgres::PoolError> {
		Ok(T::try_from_row(
			conn.query_builder_client()
				.await?
				.as_ref()
				.query_one(self.query, &params(self.params))
				.await?,
		)?)
	}

	pub async fn fetch_optional(self, conn: impl ClientLike) -> Result<Option<O>, deadpool_postgres::PoolError> {
		Ok(conn
			.query_builder_client()
			.await?
			.as_ref()
			.query_opt(self.query, &params(self.params))
			.await?
			.map(T::try_from_row)
			.transpose()?)
	}

	pub async fn fetch_many(
		self,
		conn: impl ClientLike,
	) -> Result<impl Stream<Item = Result<O, deadpool_postgres::PoolError>> + Send + Sync, deadpool_postgres::PoolError> {
		Ok(conn
			.query_builder_client()
			.await?
			.as_ref()
			.query_raw(self.query, params(self.params).into_iter())
			.await?
			.map(|row| Ok(T::try_from_row(row?)?)))
	}
}

pub trait SqlScalar {
	fn from_row(row: &Row) -> Result<Self, Error>
	where
		Self: Sized;
}

macro_rules! impl_sql_scalar {
    ($($ty:ident),*) => {
        #[allow(unused_parens)]
        impl<$($ty),*> SqlScalar for ($($ty),*,)
        where
            $($ty: for<'a> FromSql<'a>),*
        {
            #[allow(non_snake_case)]
            #[allow(unused_assignments)]
            fn from_row(row: &Row) -> Result<Self, Error> {
                let mut i = 0;
                $(
                    let $ty = row.try_get::<_, $ty>(i)?;
                    i += 1;
                )*

                Ok(($($ty),*,))
            }
        }
    };
}

macro_rules! impl_recursive {
    // Match for a single type
    ($ty:ident) => {
        impl_sql_scalar!($ty);
    };

    // Match for multiple types
    ($first:ident, $($rest:ident),+) => {
        // Recursively call for the rest of the types
        impl_sql_scalar!($first, $($rest),*);
        impl_recursive!($($rest),*);
    };
}

impl_recursive!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T16);

pub struct Separated<'b, 'args> {
	sep: &'b str,
	first: bool,
	query_builder: &'b mut QueryBuilder<'args>,
}

impl<'args> Separated<'_, 'args> {
	pub fn push_bind(&mut self, param: impl ToSql + Send + Sync + 'args) -> &mut Self {
		if self.first {
			self.first = false;
		} else {
			self.query_builder.push(self.sep);
		}

		self.query_builder.push_bind(param);
		self
	}

	pub fn push(&mut self, query: impl AsRef<str>) -> &mut Self {
		if self.first {
			self.first = false;
		} else {
			self.query_builder.push(self.sep);
		}

		self.query_builder.push(query.as_ref());
		self
	}

	pub fn push_unseparated(&mut self, query: impl AsRef<str>) -> &mut Self {
		self.query_builder.push(query.as_ref());
		self
	}

	pub fn push_bind_unseparated(&mut self, param: impl ToSql + Send + Sync + 'args) -> &mut Self {
		self.query_builder.push_bind(param);
		self
	}
}
