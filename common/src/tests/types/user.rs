use crate::types::user;

#[test]
fn test_verify_password() {
    let password = "mypassword";
    let hash =
        "$argon2id$v=19$m=16,t=2,p=1$MTJ0bmNqYzhDaXZsS1BkZw$/PWmjCzvNhg1aeUVaV8Z9w".to_string();

    assert!(user::Model {
        password_hash: hash,
        ..Default::default()
    }
    .verify_password(password));
}

#[test]
fn test_hash_password() {
    let password = "mypassword";
    let hash = user::hash_password(password);

    assert!(user::Model {
        password_hash: hash,
        ..Default::default()
    }
    .verify_password(password));
}

#[test]
fn test_validate_usernames() {
    let tests = vec![
        ("test", Ok(())),
        ("test_", Ok(())),
        ("test123", Ok(())),
        ("test123_", Ok(())),
        (
            "&garbage",
            Err("Username must only contain alphanumeric characters and underscores"),
        ),
        ("te", Err("Username must be at least 3 characters long")),
        ("tes", Ok(())),
        (
            "1111111111111111111111111111111111111111111111111111111111111",
            Err("Username must be at most 20 characters long"),
        ),
        (
            "test!",
            Err("Username must only contain alphanumeric characters and underscores"),
        ),
    ];

    for (username, result) in tests {
        assert_eq!(
            user::validate_username(username),
            result,
            "username: {}",
            username
        );
    }
}

#[test]
fn test_validate_password() {
    let tests = vec![
        ("123", Err("Password must be at least 8 characters long")),
        ("12345678", Err("Password must contain at least one lowercase character")),
        ("1234567c", Err("Password must contain at least one uppercase character")),
        ("abcdefgH", Err("Password must contain at least one digit")),
        ("1bcdefgH", Err("Password must contain at least one special character")),
        ("1!cdefgH", Ok(())),
        ("1!cdefgH111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111", Err("Password must be at most 100 characters long"))
    ];

    for (password, result) in tests {
        assert_eq!(
            user::validate_password(password),
            result,
            "pasword: {}",
            password
        );
    }
}

#[test]
fn test_validate_email() {
    let tests = vec![
        ("test", Err("Email must be at least 5 characters long")),
        ("testa", Err("Email must contain an @")),
        ("testa@", Err("Email must contain a .")),
        ("testa@.", Err("Email is not a valid email address")),
        ("testa@.com", Err("Email is not a valid email address")),
        ("testa@c.om", Ok(())),
        ("testa@abc.com", Ok(())),
    ];

    for (email, result) in tests {
        assert_eq!(user::validate_email(email), result, "email: {}", email);
    }
}
