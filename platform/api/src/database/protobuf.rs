#[derive(Debug, Clone, Default)]
pub enum ProtobufValue<T: prost::Message + std::default::Default> {
    #[default]
    None,
    Some(T),
    Err(prost::DecodeError),
}

impl<T: prost::Message + std::default::Default> ProtobufValue<T> {
    #[allow(dead_code)]
    pub fn unwrap(self) -> Option<T> {
        match self {
            Self::Some(data) => Some(data),
            Self::None => None,
            Self::Err(err) => panic!(
                "called `ProtobufValue::unwrap()` on a `Err` value: {:?}",
                err
            ),
        }
    }
}

impl<T: prost::Message + std::default::Default, F> From<Option<F>> for ProtobufValue<T>
where
    ProtobufValue<T>: From<F>,
{
    fn from(data: Option<F>) -> Self {
        match data {
            Some(data) => Self::from(data),
            None => Self::None,
        }
    }
}

impl<T: prost::Message + std::default::Default> From<Vec<u8>> for ProtobufValue<T> {
    fn from(data: Vec<u8>) -> Self {
        match T::decode(data.as_slice()) {
            Ok(variants) => Self::Some(variants),
            Err(e) => Self::Err(e),
        }
    }
}

impl<T: prost::Message + std::default::Default + PartialEq> PartialEq for ProtobufValue<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, Self::None) => true,
            (Self::Some(a), Self::Some(b)) => a == b,
            _ => false,
        }
    }
}

impl<T: prost::Message + std::default::Default + PartialEq> PartialEq<Option<T>>
    for ProtobufValue<T>
{
    fn eq(&self, other: &Option<T>) -> bool {
        match (self, other) {
            (Self::None, None) => true,
            (Self::Some(a), Some(b)) => a == b,
            _ => false,
        }
    }
}
