use async_graphql::{CustomValidator, InputValueError};

pub struct PasswordValidator;

impl CustomValidator<String> for PasswordValidator {
    fn check(&self, value: &String) -> Result<(), InputValueError<String>> {
        if value.len() < 8 {
            return Err(InputValueError::custom(
                "Password must be at least 8 characters long",
            ));
        }

        if !value.chars().any(|c| c.is_ascii_lowercase()) {
            return Err(InputValueError::custom(
                "Password must contain at least one lowercase character",
            ));
        }

        if !value.chars().any(|c| c.is_ascii_uppercase()) {
            return Err(InputValueError::custom(
                "Password must contain at least one uppercase character",
            ));
        }

        if !value.chars().any(|c| c.is_ascii_digit()) {
            return Err(InputValueError::custom(
                "Password must contain at least one digit",
            ));
        }

        if !value.chars().any(|c| !c.is_ascii_alphanumeric()) {
            return Err(InputValueError::custom(
                "Password must contain at least one special character",
            ));
        }

        if value.len() > 100 {
            return Err(InputValueError::custom(
                "Password must be at most 100 characters long",
            ));
        }

        Ok(())
    }
}
