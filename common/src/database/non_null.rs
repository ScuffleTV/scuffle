#[derive(Default)]
pub struct PgNonNullVec<T>(Vec<T>);

impl<'r, T> sqlx::Decode<'r, sqlx::Postgres> for PgNonNullVec<T>
where
	Vec<Option<T>>: sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>,
{
	fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
		let vec: Vec<Option<T>> = sqlx::Decode::decode(value)?;
		Ok(PgNonNullVec(vec.into_iter().flatten().collect()))
	}
}

impl<T: Clone> Clone for PgNonNullVec<T> {
	fn clone(&self) -> Self {
		PgNonNullVec(self.0.clone())
	}
}

impl<T: std::fmt::Debug> std::fmt::Debug for PgNonNullVec<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl<'r, T> sqlx::Type<sqlx::Postgres> for PgNonNullVec<T>
where
	Vec<T>: sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>,
{
	fn type_info() -> sqlx::postgres::PgTypeInfo {
		<Vec<T> as sqlx::Type<sqlx::Postgres>>::type_info()
	}
}

impl<T> std::ops::Deref for PgNonNullVec<T> {
	type Target = Vec<T>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T> std::ops::DerefMut for PgNonNullVec<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<T> std::iter::FromIterator<T> for PgNonNullVec<T> {
	fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
		PgNonNullVec(iter.into_iter().collect())
	}
}

impl<T> std::iter::IntoIterator for PgNonNullVec<T> {
	type IntoIter = std::vec::IntoIter<T>;
	type Item = T;

	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter()
	}
}

impl<T> std::iter::Extend<T> for PgNonNullVec<T> {
	fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
		self.0.extend(iter)
	}
}

impl<T> PgNonNullVec<T> {
	pub fn into_inner(self) -> Vec<T> {
		self.0
	}
}
