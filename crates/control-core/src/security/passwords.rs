use crate::error::{Error, Result};
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
pub fn validate(password: &str) -> Result<()> {
    if !(12..=256).contains(&password.chars().count())
        || password.chars().any(char::is_whitespace)
        || !password.chars().any(char::is_lowercase)
        || !password.chars().any(char::is_uppercase)
        || !password.chars().any(|c| c.is_ascii_digit())
    {
        return Err(Error::new("invalid_password", 422));
    }
    Ok(())
}
pub fn hash(password: &str) -> Result<String> {
    validate(password)?;
    Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|h| h.to_string())
        .map_err(|_| Error::new("password_hash_unavailable", 503))
}
pub fn verify(password: &str, hash: &str) -> bool {
    if password.len() > 1024 {
        return false;
    }
    PasswordHash::new(hash).ok().is_some_and(|h| {
        Argon2::default()
            .verify_password(password.as_bytes(), &h)
            .is_ok()
    })
}
