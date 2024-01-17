mod protobuf;
mod query_builder;

pub use deadpool_postgres::Pool;
pub use postgres_from_row::FromRow;
pub use postgres_types::Json;
pub use protobuf::*;
pub use query_builder::*;
pub use {deadpool_postgres, postgres_from_row, postgres_types, tokio_postgres};

#[inline]
pub fn json<T>(row: Json<T>) -> T {
	row.0
}

#[inline]
pub fn non_null_vec<T>(vec: Vec<Option<T>>) -> Vec<T> {
	vec.into_iter().flatten().collect()
}
