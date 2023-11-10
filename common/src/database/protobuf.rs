#[repr(transparent)]
pub struct Protobuf<T>(pub T);

impl<T> Clone for Protobuf<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub trait TraitProtobuf<T> {
    fn into_inner(self) -> T;
}

pub trait TraitProtobufVec<T> {
    fn into_vec(self) -> Vec<T>;
}

impl<T> TraitProtobuf<T> for Protobuf<T> {
    fn into_inner(self) -> T {
        self.0
    }
}

impl<T> TraitProtobufVec<T> for Vec<Protobuf<T>> {
    fn into_vec(self) -> Vec<T> {
        self.into_iter().map(|a| a.into_inner()).collect()
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Protobuf<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: std::default::Default> Default for Protobuf<T> {
    fn default() -> Self {
        Self(T::default())
    }
}

impl<T> sqlx::Type<sqlx::Postgres> for Protobuf<T> {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <Vec<u8> as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl<T> sqlx::postgres::PgHasArrayType for Protobuf<T> {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        <Vec<u8> as sqlx::postgres::PgHasArrayType>::array_type_info()
    }
}

impl<T: prost::Message> sqlx::Encode<'_, sqlx::Postgres> for Protobuf<T> {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
        <Vec<u8> as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&self.0.encode_to_vec(), buf)
    }
}

impl<T: prost::Message + std::default::Default> sqlx::Decode<'_, sqlx::Postgres> for Protobuf<T> {
    fn decode(
        value: sqlx::postgres::PgValueRef<'_>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let bytes = <Vec<u8> as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        let inner = T::decode(bytes.as_slice())?;
        Ok(Self(inner))
    }
}

impl<T> AsRef<T> for Protobuf<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for Protobuf<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> std::ops::Deref for Protobuf<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Protobuf<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for Protobuf<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}
