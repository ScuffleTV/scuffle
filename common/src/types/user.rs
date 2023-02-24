use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub id: i64,                      // bigint, primary key
    pub username: String,             // varchar(32)
    pub password_hash: String,        // varchar(255)
    pub email: String,                // varchar(255)
    pub email_verified: bool,         // bool
    pub created_at: DateTime<Utc>,    // timestamptz
    pub last_login_at: DateTime<Utc>, // timestamptz
}

impl Model {
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
}

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);

    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("failed to hash password");

    hash.to_string()
}

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
