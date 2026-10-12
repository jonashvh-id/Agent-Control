use control_core::error::{Error, Result};
use std::path::PathBuf;
#[derive(Clone)]
pub struct Config {
    pub repositories: Vec<String>,
    pub token_file: Option<PathBuf>,
    pub credential_source: String,
    pub timeout: std::time::Duration,
}
impl Config {
    pub fn validate(&self, _loopback: bool) -> Result<()> {
        let re =
            regex::Regex::new(r"^[A-Za-z0-9][A-Za-z0-9-]{0,38}/[A-Za-z0-9_.-]{1,100}$").unwrap();
        if self.repositories.is_empty()
            || self.repositories.len() > 20
            || self.repositories.iter().any(|r| {
                !re.is_match(r)
                    || matches!(r.split('/').nth(1), Some("." | ".."))
            })
            || self.timeout.is_zero()
            || self.timeout.as_secs_f64() > 15.
        {
            return Err(Error::new("invalid_github_configuration", 400));
        }
        if self.credential_source != "file" || self.token_file.is_none() {
            return Err(Error::new("invalid_github_credentials", 400));
        }
        Ok(())
    }
    pub fn permits(&self, repository: &str) -> bool {
        self.repositories
            .iter()
            .any(|r| r.eq_ignore_ascii_case(repository))
    }
}
