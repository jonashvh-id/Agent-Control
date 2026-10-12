use crate::Config;
use control_core::{
    error::{Error, Result},
    security::files::private_read,
};
use zeroize::Zeroizing;
pub fn token(config: &Config) -> Result<Zeroizing<String>> {
    let unavailable = || Error::new("google_credential_unavailable", 503);
    let raw = Zeroizing::new(private_read(&config.token_file, 4097).map_err(|_| unavailable())?);
    let value = std::str::from_utf8(&raw).map_err(|_| unavailable())?.trim();
    if value.is_empty() || value.len() > 4096 || value.chars().any(char::is_whitespace) {
        return Err(unavailable());
    }
    Ok(Zeroizing::new(value.into()))
}
