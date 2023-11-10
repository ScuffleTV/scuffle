#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Ulid(pub ulid::Ulid);

impl sqlx::postgres::PgHasArrayType for Ulid {
    fn array_compatible(ty: &sqlx_postgres::PgTypeInfo) -> bool {
        <uuid::Uuid as sqlx::postgres::PgHasArrayType>::array_compatible(ty)
    }

    fn array_type_info() -> sqlx_postgres::PgTypeInfo {
        <uuid::Uuid as sqlx::postgres::PgHasArrayType>::array_type_info()
    }
}

impl Default for Ulid {
    fn default() -> Self {
        Self(ulid::Ulid::nil())
    }
}

impl std::fmt::Display for Ulid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.to_string().fmt(f)
    }
}

impl std::fmt::Debug for Ulid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl sqlx::Decode<'_, sqlx::Postgres> for Ulid {
    fn decode(
        value: sqlx::postgres::PgValueRef<'_>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let id = <uuid::Uuid as sqlx::Decode<'_, sqlx::Postgres>>::decode(value)?;
        Ok(Ulid(ulid::Ulid::from(id)))
    }
}

impl sqlx::Encode<'_, sqlx::Postgres> for Ulid {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
        <uuid::Uuid as sqlx::Encode<'_, sqlx::Postgres>>::encode_by_ref(&self.0.into(), buf)
    }
}

impl sqlx::Type<sqlx::Postgres> for Ulid {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <uuid::Uuid as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl From<Ulid> for ulid::Ulid {
    fn from(id: Ulid) -> Self {
        id.0
    }
}

impl From<ulid::Ulid> for Ulid {
    fn from(id: ulid::Ulid) -> Self {
        Ulid(id)
    }
}

impl From<uuid::Uuid> for Ulid {
    fn from(id: uuid::Uuid) -> Self {
        Ulid(ulid::Ulid::from(id))
    }
}

impl From<Ulid> for uuid::Uuid {
    fn from(id: Ulid) -> Self {
        id.0.into()
    }
}
