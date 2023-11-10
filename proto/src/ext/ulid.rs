use crate::scuffle::types::Ulid;

pub trait UlidExt {
    fn to_ulid(&self) -> ulid::Ulid;
    fn to_uuid(&self) -> uuid::Uuid {
        self.to_ulid().into()
    }
}

impl UlidExt for Ulid {
    fn to_ulid(&self) -> ulid::Ulid {
        ulid::Ulid::from((self.msb, self.lsb))
    }
}

impl UlidExt for Option<Ulid> {
    fn to_ulid(&self) -> ulid::Ulid {
        match self {
            Some(ulid) => ulid.to_ulid(),
            None => ulid::Ulid::nil(),
        }
    }
}

impl From<uuid::Uuid> for Ulid {
    fn from(uuid: uuid::Uuid) -> Self {
        let (msb, lsb) = uuid.as_u64_pair();
        Self { msb, lsb }
    }
}

impl From<ulid::Ulid> for Ulid {
    fn from(uuid: ulid::Ulid) -> Self {
        let msb = (uuid.0 >> 64) as u64;
        let lsb = uuid.0 as u64;
        Self { msb, lsb }
    }
}
