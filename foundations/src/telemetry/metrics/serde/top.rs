use prometheus_client::encoding::{EncodeLabelKey, LabelSetEncoder};
use serde::ser::{Impossible, Serialize, SerializeStruct, Serializer};

use super::value;

#[inline]
pub(super) fn serializer(writer: LabelSetEncoder<'_>) -> TopSerializer<'_> {
	TopSerializer { writer }
}

pub(super) struct TopSerializer<'w> {
	writer: LabelSetEncoder<'w>,
}

macro_rules! unsupported_scalars {
    ($($($method:ident: $ty:ty),+ $(,)?)?) => {$($(
        #[inline]
        fn $method(self, _: $ty) -> Result<Self::Ok, Self::Error> {
            Err(Self::Error::Unexpected(format!("scalar type: {}", stringify!($ty))))
        }
    )+)?}
}

impl<'w> Serializer for TopSerializer<'w> {
	type Error = super::Error;
	type Ok = ();
	type SerializeMap = Impossible<Self::Ok, Self::Error>;
	type SerializeSeq = Impossible<Self::Ok, Self::Error>;
	type SerializeStruct = StructSerializer<'w>;
	type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;
	type SerializeTuple = Impossible<Self::Ok, Self::Error>;
	type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
	type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;

	unsupported_scalars! {
		serialize_bool: bool,
		serialize_i8: i8,
		serialize_i16: i16,
		serialize_i32: i32,
		serialize_i64: i64,
		serialize_u8: u8,
		serialize_u16: u16,
		serialize_u32: u32,
		serialize_u64: u64,
		serialize_f32: f32,
		serialize_f64: f64,
		serialize_char: char,
		serialize_str: &str,
		serialize_bytes: &[u8],
	}

	#[inline]
	fn serialize_unit(self) -> Result<(), Self::Error> {
		Ok(())
	}

	#[inline]
	fn serialize_unit_struct(self, _name: &'static str) -> Result<(), Self::Error> {
		Ok(())
	}

	#[inline]
	fn serialize_unit_variant(self, ty: &'static str, _index: u32, name: &'static str) -> Result<(), Self::Error> {
		Err(Self::Error::Unexpected(format!("unit variant: {ty}::{name}")))
	}

	#[inline]
	fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<(), Self::Error>
	where
		T: ?Sized + Serialize,
	{
		value.serialize(self)
	}

	#[inline]
	fn serialize_newtype_variant<T>(
		self,
		ty: &'static str,
		_index: u32,
		name: &'static str,
		_value: &T,
	) -> Result<(), Self::Error>
	where
		T: ?Sized + Serialize,
	{
		Err(Self::Error::Unexpected(format!("newtype variant: {ty}::{name}")))
	}

	#[inline]
	fn serialize_none(self) -> Result<(), Self::Error> {
		Ok(())
	}

	#[inline]
	fn serialize_some<T>(self, value: &T) -> Result<(), Self::Error>
	where
		T: ?Sized + Serialize,
	{
		value.serialize(self)
	}

	#[inline]
	fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
		Err(Self::Error::Unexpected(format!("sequence: {:?}", len)))
	}

	#[inline]
	fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
		Err(Self::Error::Unexpected(format!("tuple: {:?}", len)))
	}

	#[inline]
	fn serialize_tuple_struct(self, ty: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
		Err(Self::Error::Unexpected(format!("tuple struct: {ty}")))
	}

	#[inline]
	fn serialize_tuple_variant(
		self,
		ty: &'static str,
		_index: u32,
		name: &'static str,
		_len: usize,
	) -> Result<Self::SerializeTupleVariant, Self::Error> {
		Err(Self::Error::Unexpected(format!("tuple variant: {ty}::{name}")))
	}

	#[inline]
	fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
		Err(Self::Error::Unexpected(format!("map: {:?}", len)))
	}

	#[inline]
	fn serialize_struct(self, _ty: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> {
		Ok(StructSerializer(self.writer))
	}

	#[inline]
	fn serialize_struct_variant(
		self,
		ty: &'static str,
		_index: u32,
		name: &'static str,
		_len: usize,
	) -> Result<Self::SerializeStructVariant, Self::Error> {
		Err(Self::Error::Unexpected(format!("struct variant: {ty}::{name}")))
	}
}

pub(super) struct StructSerializer<'w>(LabelSetEncoder<'w>);

impl SerializeStruct for StructSerializer<'_> {
	type Error = super::Error;
	type Ok = ();

	fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
	where
		T: ?Sized + Serialize,
	{
		let mut encoder = self.0.encode_label();
		let mut label_enc = encoder.encode_label_key().map_err(super::Error::Fmt)?;
		key.encode(&mut label_enc).map_err(super::Error::Fmt)?;
		let value_enc = label_enc.encode_label_value().map_err(super::Error::Fmt)?;
		value.serialize(value::serializer(value_enc))?;

		Ok(())
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}
