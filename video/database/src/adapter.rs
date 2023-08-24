#[repr(transparent)]
pub struct Adapter<T>(pub T);

impl<T> Clone for Adapter<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub trait TraitAdapter<T> {
    fn into_inner(self) -> T;
}

pub trait TraitAdapterVec<T> {
    fn into_vec(self) -> Vec<T>;
}

impl<T> TraitAdapter<T> for Adapter<T> {
    fn into_inner(self) -> T {
        self.0
    }
}

impl<T> TraitAdapterVec<T> for Vec<Adapter<T>> {
    fn into_vec(self) -> Vec<T> {
        self.into_iter().map(|a| a.into_inner()).collect()
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Adapter<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: std::default::Default> Default for Adapter<T> {
    fn default() -> Self {
        Self(T::default())
    }
}

impl<T> sqlx::Type<sqlx::Postgres> for Adapter<T> {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <Vec<u8> as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl<T> sqlx::postgres::PgHasArrayType for Adapter<T> {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        <Vec<u8> as sqlx::postgres::PgHasArrayType>::array_type_info()
    }
}

impl<T: prost::Message> sqlx::Encode<'_, sqlx::Postgres> for Adapter<T> {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
        <Vec<u8> as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&self.0.encode_to_vec(), buf)
    }
}

impl<T: prost::Message + std::default::Default> sqlx::Decode<'_, sqlx::Postgres> for Adapter<T> {
    fn decode(
        value: sqlx::postgres::PgValueRef<'_>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let bytes = <Vec<u8> as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        let inner = T::decode(bytes.as_slice())?;
        Ok(Self(inner))
    }
}

impl<T> AsRef<T> for Adapter<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for Adapter<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> std::ops::Deref for Adapter<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Adapter<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for Adapter<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}
