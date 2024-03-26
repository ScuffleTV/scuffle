use bytes::BytesMut;
use postgres_types::{accepts, to_sql_checked, IsNull};

#[derive(Debug, Clone)]
pub struct Protobuf<T>(pub T);

impl<T: prost::Message + Default> postgres_types::FromSql<'_> for Protobuf<T> {
	accepts!(BYTEA);

	fn from_sql(_ty: &postgres_types::Type, raw: &[u8]) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
		Ok(Self(T::decode(raw)?))
	}
}

impl<T: prost::Message> postgres_types::ToSql for Protobuf<T> {
	to_sql_checked!();

	fn to_sql(
		&self,
		ty: &postgres_types::Type,
		w: &mut BytesMut,
	) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
		<&[u8] as postgres_types::ToSql>::to_sql(&&*self.0.encode_to_vec(), ty, w)
	}

	fn accepts(ty: &postgres_types::Type) -> bool {
		<&[u8] as postgres_types::ToSql>::accepts(ty)
	}
}

#[inline]
pub fn protobuf<T>(row: Protobuf<T>) -> T {
	row.0
}

#[inline]
pub fn protobuf_vec<T>(row: Vec<Protobuf<T>>) -> Vec<T> {
	row.into_iter().map(|protobuf| protobuf.0).collect()
}

#[inline]
pub fn protobuf_opt<T>(row: Option<Protobuf<T>>) -> Option<T> {
	row.map(protobuf)
}

#[inline]
pub fn protobuf_vec_opt<T>(row: Option<Vec<Protobuf<T>>>) -> Option<Vec<T>> {
	row.map(protobuf_vec)
}
