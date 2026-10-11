use std::fmt;
#[derive(Debug, Clone)]
pub struct Error {
    pub code: &'static str,
    pub status: u16,
    pub uncertain: bool,
}
pub type Result<T> = std::result::Result<T, Error>;
impl Error {
    pub fn new(code: &'static str, status: u16) -> Self {
        Self {
            code,
            status,
            uncertain: false,
        }
    }
    pub fn uncertain(code: &'static str, status: u16) -> Self {
        Self {
            code,
            status,
            uncertain: true,
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}
impl std::error::Error for Error {}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::new("storage_unavailable", 503)
    }
}
impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Self::new("runtime_unavailable", 503)
    }
}
