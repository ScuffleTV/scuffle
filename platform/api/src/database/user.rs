use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{DateTime, Utc};
use rand::Rng;
use ulid::Ulid;

use super::Channel;

#[derive(PartialEq, Eq, Clone, Debug, thiserror::Error)]
pub enum TotpError {
	#[error("cannot find secret, please generate a secret first")]
	NoSecret,
	#[error("failed to initilize totp")]
	Initilize,
	#[error("failed to generate totp code")]
	Generate,
}

#[derive(Debug, Clone, Default, postgres_from_row::FromRow)]
pub struct User {
	/// The unique identifier for the user.
	pub id: Ulid,
	/// The username of the user.
	pub username: String,
	/// The display name of the user.
	pub display_name: String,
	/// The profile picture of the user.
	pub profile_picture_id: Option<Ulid>,
	/// Pending profile picture of the user. This is used when a user uploads a
	/// new profile picture and its still being processed.
	pub pending_profile_picture_id: Option<Ulid>,
	/// The display color of the user.
	pub display_color: i32,
	/// The hashed password of the user. (argon2)
	pub password_hash: String,
	/// Whether two factor authentication is enabled for the user.
	pub totp_enabled: bool,
	/// The secret used for two factor authentication.
	pub totp_secret: Option<Vec<u8>>,
	/// The backup codes used for two factor authentication.
	pub two_fa_backup_codes: Option<Vec<i32>>,
	/// The email of the user.
	pub email: String,
	/// Whether the user has verified their email.
	pub email_verified: bool,
	/// The time the user last logged in.
	pub last_login_at: DateTime<Utc>,
	/// The time the user was last updated.
	pub updated_at: DateTime<Utc>,
	/// The roles of the user.
	pub roles: Vec<Ulid>,

	/// Channel
	#[from_row(flatten)]
	pub channel: Channel,
}

impl User {
	/// Uses argon2 to verify the password hash against the provided password.
	pub fn verify_password(&self, password: &str) -> bool {
		let hash = match PasswordHash::new(&self.password_hash) {
			Ok(hash) => hash,
			Err(err) => {
				tracing::error!("failed to parse password hash: {}", err);
				return false;
			}
		};

		Argon2::default().verify_password(password.as_bytes(), &hash).is_ok()
	}

	pub fn verify_totp_code(&self, code: &str, backup_codes: bool) -> Result<bool, TotpError> {
		// TODO: Remove backup code when used
		let totp_secret = self.totp_secret.clone().ok_or(TotpError::NoSecret)?;
		let rfc = totp_rs::Rfc6238::with_defaults(totp_secret).map_err(|_| TotpError::Initilize)?;
		let totp = totp_rs::TOTP::from_rfc6238(rfc).unwrap();

		if totp.generate_current().map_err(|_| TotpError::Generate)? == code {
			return Ok(true);
		} else if backup_codes {
			// Check backup codes.
			if let Some(two_fa_backup_codes) = &self.two_fa_backup_codes {
				if let Ok(code) = u32::from_str_radix(code, 16) {
					if two_fa_backup_codes.contains(&(code as i32)) {
						return Ok(true);
					}
				}
			};
		}

		Ok(false)
	}

	/// Generates a new password hash using argon2.
	pub fn hash_password(password: &str) -> String {
		let salt = SaltString::generate(&mut OsRng);

		let hash = Argon2::default()
			.hash_password(password.as_bytes(), &salt)
			.expect("failed to hash password");

		hash.to_string()
	}

	/// https://www.rapidtables.com/convert/color/hsl-to-rgb.html
	fn hsl_to_rgb(h: u16, s: f64, l: f64) -> (u8, u8, u8) {
		let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
		let x = c * (1.0 - ((h as f64 / 60.0) % 2.0 - 1.0).abs());
		let m = l - c / 2.0;
		let (r, g, b) = match h {
			0..=59 => (c, x, 0.0),
			60..=119 => (x, c, 0.0),
			120..=179 => (0.0, c, x),
			180..=239 => (0.0, x, c),
			240..=299 => (x, 0.0, c),
			300..=359 => (c, 0.0, x),
			_ => (0.0, 0.0, 0.0),
		};

		(((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8)
	}

	pub fn generate_display_color() -> i32 {
		let (r, g, b) = Self::hsl_to_rgb(rand::thread_rng().gen_range(0..=359), 1.0, 0.67);
		((r as i32) << 16) + ((g as i32) << 8) + b as i32
	}
}
