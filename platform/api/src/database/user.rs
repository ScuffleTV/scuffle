use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use chrono::{DateTime, Utc};
use rand::Rng;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
#[repr(i32)]
pub enum LiveState {
    #[default]
    NotLive = 0,
    Live = 1,
    LiveReady = 2,
}

impl From<i32> for LiveState {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::NotLive,
            1 => Self::Live,
            2 => Self::LiveReady,
            _ => Self::NotLive,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Model {
    /// The unique identifier for the user.
    pub id: Uuid,
    /// The username of the user.
    pub username: String,
    /// The display name of the user.
    pub display_name: String,
    /// The hashed password of the user. (argon2)
    pub password_hash: String,
    /// The email of the user.
    pub email: String,
    /// Whether the user has verified their email.
    pub email_verified: bool,
    /// The time the user was created.
    pub created_at: DateTime<Utc>,
    /// The time the user last logged in.
    pub last_login_at: DateTime<Utc>,
    /// The stream key of the user.
    pub stream_key: String,
    /// The title of the stream
    pub stream_title: String,
    /// The description of the stream
    pub stream_description: String,
    /// Whether the stream transcoding is enabled
    pub stream_transcoding_enabled: bool,
    /// Whether the stream recording is enabled
    pub stream_recording_enabled: bool,
}

impl Model {
    /// Uses argon2 to verify the password hash against the provided password.
    pub fn verify_password(&self, password: &str) -> bool {
        let hash = match PasswordHash::new(&self.password_hash) {
            Ok(hash) => hash,
            Err(err) => {
                tracing::error!("failed to parse password hash: {}", err);
                return false;
            }
        };

        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    }

    pub fn get_stream_key(&self) -> String {
        format!("live_{}_{}", self.id.as_u128(), self.stream_key)
    }
}

/// Generates a new password hash using argon2.
pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);

    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("failed to hash password");

    hash.to_string()
}

/// Validates a username.
pub fn validate_username(username: &str) -> Result<(), &'static str> {
    if username.len() < 3 {
        return Err("Username must be at least 3 characters long");
    }

    if username.len() > 20 {
        return Err("Username must be at most 20 characters long");
    }

    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err("Username must only contain alphanumeric characters and underscores");
    }

    Ok(())
}

/// Validates a password.
pub fn validate_password(password: &str) -> Result<(), &'static str> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters long");
    }

    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        return Err("Password must contain at least one lowercase character");
    }

    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        return Err("Password must contain at least one uppercase character");
    }

    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err("Password must contain at least one digit");
    }

    if !password.chars().any(|c| !c.is_ascii_alphanumeric()) {
        return Err("Password must contain at least one special character");
    }

    if password.len() > 100 {
        return Err("Password must be at most 100 characters long");
    }

    Ok(())
}

/// Validates an email.
pub fn validate_email(email: &str) -> Result<(), &'static str> {
    if email.len() < 5 {
        return Err("Email must be at least 5 characters long");
    }

    if email.len() > 100 {
        return Err("Email must be at most 100 characters long");
    }

    if !email.contains('@') {
        return Err("Email must contain an @");
    }

    if !email.contains('.') {
        return Err("Email must contain a .");
    }

    if !email_address::EmailAddress::is_valid(email) {
        return Err("Email is not a valid email address");
    }

    Ok(())
}

/// Generates a new stream key.
pub fn generate_stream_key() -> String {
    let mut rng = rand::thread_rng();
    let mut key = String::new();

    for _ in 0..24 {
        key.push(rng.sample(rand::distributions::Alphanumeric).into());
    }

    key
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

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

pub fn generate_display_color() -> i32 {
    let (r, g, b) = hsl_to_rgb(rand::thread_rng().gen_range(0..=359), 1.0, 0.67);
    ((r as i32) << 16) + ((g as i32) << 8) + b as i32
}
