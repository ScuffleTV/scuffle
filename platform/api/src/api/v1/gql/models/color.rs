use std::ops::Deref;

use async_graphql::{ComplexObject, InputValueError, InputValueResult, Scalar, ScalarType, SimpleObject, Value};

/// A hex rgb color code.
#[derive(Copy, Clone, Debug)]
pub struct RgbColor(i32);

#[Scalar]
impl ScalarType for RgbColor {
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

impl RgbColor {
	fn split(&self) -> (u8, u8, u8) {
		let r = (self.0 >> 16) & 0xFF;
		let g = (self.0 >> 8) & 0xFF;
		let b = self.0 & 0xFF;
		(r as u8, g as u8, b as u8)
	}
}

impl Deref for RgbColor {
	type Target = i32;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl From<i32> for RgbColor {
	fn from(value: i32) -> Self {
		Self(value)
	}
}

impl ToString for RgbColor {
	fn to_string(&self) -> String {
		format!("#{:06x}", self.0)
	}
}

/// A HSL color.
#[derive(Copy, Clone, Debug, SimpleObject)]
pub struct HslColor {
	/// Hue in degrees, value between 0.0 and 360.0.
	pub h: f64,
	/// Saturation, value between 0.0 and 1.0.
	pub s: f64,
	/// Lightness, value between 0.0 and 1.0.
	pub l: f64,
}

#[derive(SimpleObject, Clone)]
#[graphql(complex)]
pub struct DisplayColor {
	pub rgb: RgbColor,
}

impl From<RgbColor> for DisplayColor {
	fn from(color: RgbColor) -> Self {
		Self { rgb: color }
	}
}

impl From<i32> for DisplayColor {
	fn from(color: i32) -> Self {
		Self {
			rgb: RgbColor::from(color),
		}
	}
}

#[ComplexObject]
impl DisplayColor {
	// https://www.rapidtables.com/convert/color/rgb-to-hsl.html
	async fn hsl(&self) -> HslColor {
		let (r, g, b) = self.rgb.split();
		let r = r as f64 / 255.0;
		let g = g as f64 / 255.0;
		let b = b as f64 / 255.0;

		let c_max = r.max(g).max(b);
		let c_min = r.min(g).min(b);
		let delta = c_max - c_min;

		let mut h = if delta == 0.0 {
			0.0
		} else if c_max == r {
			60.0 * (((g - b) / delta) % 6.0)
		} else if c_max == g {
			60.0 * ((b - r) / delta + 2.0)
		} else {
			60.0 * ((r - g) / delta + 4.0)
		};

		if h < 0.0 {
			h += 360.0;
		}

		let l = (c_max + c_min) / 2.0;

		let s = if delta == 0.0 {
			0.0
		} else {
			delta / (1.0 - (2.0 * l - 1.0).abs())
		};

		HslColor { h, s, l }
	}

	async fn is_gray(&self) -> bool {
		let (r, g, b) = self.rgb.split();
		// Color is gray when r == g == b
		r == g && g == b
	}
}
