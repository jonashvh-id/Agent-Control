use control_core::error::{Error, Result};
use std::{path::PathBuf, time::Duration};
#[derive(Clone)]
pub struct Config {
    pub files: Vec<String>,
    pub token_file: PathBuf,
    pub timeout: Duration,
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        let id = regex::Regex::new(r"^[A-Za-z0-9_-]{10,128}$").unwrap();
        if self.files.is_empty()
            || self.files.len() > 20
            || self.files.iter().any(|f| !id.is_match(f))
            || self.token_file.as_os_str().is_empty()
            || self.timeout.is_zero()
            || self.timeout.as_secs_f64() > 10.0
        {
            return Err(Error::new("invalid_drive_configuration", 400));
        }
        Ok(())
    }
    pub fn permits(&self, id: &str) -> bool {
        self.files.iter().any(|f| f == id)
    }
}
