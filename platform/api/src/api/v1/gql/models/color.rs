use std::ops::Deref;

use async_graphql::{ComplexObject, InputValueError, InputValueResult, Scalar, ScalarType, SimpleObject, Value};

/// A hex rgb color code.
#[derive(Copy, Clone, Debug)]
pub struct Color(i32);

#[Scalar]
impl ScalarType for Color {
	fn parse(value: Value) -> InputValueResult<Self> {
		match value {
			Value::String(s) => {
				let s = s.strip_prefix('#').ok_or(InputValueError::custom("Invalid value"))?;
				let r = s
					.get(0..2)
					.and_then(|r| u8::from_str_radix(r, 16).ok())
					.ok_or(InputValueError::custom("Invalid value"))?;
				let g = s
					.get(2..4)
					.and_then(|g| u8::from_str_radix(g, 16).ok())
					.ok_or(InputValueError::custom("Invalid value"))?;
				let b = s
					.get(4..6)
					.and_then(|b| u8::from_str_radix(b, 16).ok())
					.ok_or(InputValueError::custom("Invalid value"))?;
				Ok(Self((r as i32) << 16 | (g as i32) << 8 | b as i32))
			}
			_ => Err(InputValueError::custom("Invalid value")),
		}
	}

	fn to_value(&self) -> Value {
		Value::String(self.to_string())
	}
}

impl Color {
	fn rgb(&self) -> (u8, u8, u8) {
		let r = (self.0 >> 16) & 0xFF;
		let g = (self.0 >> 8) & 0xFF;
		let b = self.0 & 0xFF;
		(r as u8, g as u8, b as u8)
	}
}

impl Deref for Color {
	type Target = i32;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl From<i32> for Color {
	fn from(value: i32) -> Self {
		Self(value)
	}
}

impl ToString for Color {
	fn to_string(&self) -> String {
		let (r, g, b) = self.rgb();
		format!("#{r:02x}{g:02x}{b:02x}")
	}
}

#[derive(SimpleObject, Clone)]
#[graphql(complex)]
pub struct DisplayColor {
	pub color: Color,
}

impl From<Color> for DisplayColor {
	fn from(color: Color) -> Self {
		Self { color }
	}
}

impl From<i32> for DisplayColor {
	fn from(color: i32) -> Self {
		Self {
			color: Color::from(color),
		}
	}
}

#[ComplexObject]
impl DisplayColor {
	// https://www.rapidtables.com/convert/color/rgb-to-hsl.html
	async fn hue(&self) -> f64 {
		let (r, g, b) = self.color.rgb();
		let r = r as f64 / 255.0;
		let g = g as f64 / 255.0;
		let b = b as f64 / 255.0;

		let c_max = r.max(g).max(b);
		let c_min = r.min(g).min(b);
		let delta = c_max - c_min;

		let h = if delta == 0.0 {
			0.0
		} else if c_max == r {
			60.0 * (((g - b) / delta) % 6.0)
		} else if c_max == g {
			60.0 * ((b - r) / delta + 2.0)
		} else {
			60.0 * ((r - g) / delta + 4.0)
		};

		if h < 0.0 { h + 360.0 } else { h }
	}

	async fn is_gray(&self) -> bool {
		let (r, g, b) = self.color.rgb();
		// Color is gray when r == g == b
		r == g && g == b
	}
}
