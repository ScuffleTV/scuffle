use prometheus_client::encoding::{EncodeLabelValue, LabelValueEncoder};
use serde::ser::{Impossible, Serialize, Serializer};
use std::{fmt, str};

#[inline]
pub(super) fn serializer(
    writer: LabelValueEncoder<'_>,
) -> impl Serializer<Ok = (), Error = super::Error> + '_ {
    ValueSerializer { writer }
}

struct ValueSerializer<'w> {
    writer: LabelValueEncoder<'w>,
}

macro_rules! delegate {
    ($($method:ident: $ty:ty),*,) => {$(
        #[inline]
        fn $method(mut self, v: $ty) -> Result<Self::Ok, Self::Error> {
            v.encode(&mut self.writer).map_err(Self::Error::Fmt)
        }
    )*}
}

impl Serializer for ValueSerializer<'_> {
    type Ok = ();
    type Error = super::Error;
    type SerializeSeq = Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(mut self, v: bool) -> Result<Self::Ok, Self::Error> {
        if v {
            "true".encode(&mut self.writer).map_err(Self::Error::Fmt)?;
        } else {
            "false".encode(&mut self.writer).map_err(Self::Error::Fmt)?;
        }

        self.writer.finish().map_err(Self::Error::Fmt)?;

        Ok(())
    }

    delegate! {
        serialize_i8: i8,
        serialize_i16: i16,
        serialize_i32: i32,
        serialize_i64: i64,
        serialize_u8: u8,
        serialize_u16: u16,
        serialize_u32: u32,
        serialize_u64: u64,
        serialize_u128: u128,
        serialize_i128: i128,
        serialize_f64: f64,
    }

    fn serialize_f32(mut self, v: f32) -> Result<Self::Ok, Self::Error> {
        (v as f64)
            .encode(&mut self.writer)
            .map_err(Self::Error::Fmt)?;

        self.writer.finish().map_err(Self::Error::Fmt)?;

        Ok(())
    }

    fn serialize_char(mut self, v: char) -> Result<Self::Ok, Self::Error> {
        format!("{v}")
            .encode(&mut self.writer)
            .map_err(Self::Error::Fmt)?;

        self.writer.finish().map_err(Self::Error::Fmt)?;

        Ok(())
    }

    fn serialize_str(mut self, value: &str) -> Result<Self::Ok, Self::Error> {
        value.encode(&mut self.writer).map_err(Self::Error::Fmt)?;

        self.writer.finish().map_err(Self::Error::Fmt)?;

        Ok(())
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(Self::Error::Unexpected("bytes".to_string()))
    }

    fn serialize_unit(mut self) -> Result<Self::Ok, Self::Error> {
        None::<i32>
            .encode(&mut self.writer)
            .map_err(Self::Error::Fmt)?;
        self.writer.finish().map_err(Self::Error::Fmt)?;
        Ok(())
    }

    fn serialize_unit_struct(mut self, _ty: &'static str) -> Result<Self::Ok, Self::Error> {
        None::<i32>
            .encode(&mut self.writer)
            .map_err(Self::Error::Fmt)?;
        self.writer.finish().map_err(Self::Error::Fmt)?;
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _ty: &'static str,
        _index: u32,
        name: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(name)
    }

    fn serialize_newtype_struct<T>(
        self,
        _ty: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        ty: &'static str,
        _index: u32,
        name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(Self::Error::Unexpected(format!(
            "newtype variant: {ty}::{name}"
        )))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(Self::Error::Unexpected(format!("seq: {:?}", len)))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(Self::Error::Unexpected(format!("tuple: {:?}", len)))
    }

    fn serialize_tuple_struct(
        self,
        ty: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTuple, Self::Error> {
        Err(Self::Error::Unexpected(format!("tuple struct: {ty}")))
    }

    fn serialize_tuple_variant(
        self,
        ty: &'static str,
        _index: u32,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Self::Error::Unexpected(format!(
            "tuple variant: {ty}::{name}"
        )))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(Self::Error::Unexpected(format!("map: {:?}", len)))
    }

    fn serialize_struct(
        self,
        ty: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(Self::Error::Unexpected(format!("struct: {ty}")))
    }

    fn serialize_struct_variant(
        self,
        ty: &'static str,
        _index: u32,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Self::Error::Unexpected(format!(
            "struct variant: {ty}::{name}"
        )))
    }

    fn collect_str<T>(mut self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + fmt::Display,
    {
        value
            .to_string()
            .encode(&mut self.writer)
            .map_err(Self::Error::Fmt)?;

        self.writer.finish().map_err(Self::Error::Fmt)?;

        Ok(())
    }

    fn is_human_readable(&self) -> bool {
        true
    }
}
