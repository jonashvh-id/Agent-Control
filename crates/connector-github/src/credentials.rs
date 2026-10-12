//! The GitHub provider credential must come from an explicit, private file.
use crate::Config;
use control_core::{
    error::{Error, Result},
    security::files::private_read,
};
use zeroize::Zeroizing;
fn unavailable() -> Error {
    Error::new("github_credential_unavailable", 503)
}
pub async fn token(config: &Config) -> Result<Zeroizing<String>> {
    if config.credential_source != "file" {
        return Err(unavailable());
    }
    let path = config.token_file.as_ref().ok_or_else(unavailable)?;
    let raw = Zeroizing::new(private_read(path, 4097).map_err(|_| unavailable())?);
    let value = std::str::from_utf8(&raw).map_err(|_| unavailable())?.trim();
    if value.is_empty() || value.len() > 4096 || value.chars().any(char::is_whitespace) {
        return Err(unavailable());
    }
    Ok(Zeroizing::new(value.to_string()))
}
