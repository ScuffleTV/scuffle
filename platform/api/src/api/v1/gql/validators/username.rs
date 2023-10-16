use async_graphql::{CustomValidator, InputValueError};

pub struct UsernameValidator;

impl CustomValidator<String> for UsernameValidator {
    fn check(&self, value: &String) -> Result<(), InputValueError<String>> {
        if value.len() < 3 {
            return Err(InputValueError::custom(
                "Username must be at least 3 characters long",
            ));
        }

        if value.len() > 20 {
            return Err(InputValueError::custom(
                "Username must be at most 20 characters long",
            ));
        }

        if !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(InputValueError::custom(
                "Username must only contain alphanumeric characters and underscores",
            ));
        }

        Ok(())
    }
}
