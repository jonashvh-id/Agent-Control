use control_core::{
    error::{Error, Result},
    models::Config,
};
use std::path::PathBuf;
pub type Configuration = (
    Config,
    Option<connector_github::Config>,
    Option<connector_google_drive::Config>,
    Option<connector_google_drive::oauth::Config>,
);
pub fn from_env() -> Result<Configuration> {
    let value = |key: &str, default: &str| std::env::var(key).unwrap_or(default.into());
    let mut config = Config::local(PathBuf::from(value("AGENT_CONTROL_DATA_DIR", "var-rust")));
    config.origin = value("AGENT_CONTROL_ORIGIN", "http://127.0.0.1:8010")
        .trim_end_matches('/')
        .into();
    config.secure_cookies =
        value("AGENT_CONTROL_SECURE_COOKIES", "false").eq_ignore_ascii_case("true");
    config.execution_enabled =
        value("AGENT_CONTROL_EXECUTION_ENABLED", "false").eq_ignore_ascii_case("true");
    config.mcp_enabled = value("AGENT_CONTROL_MCP_ENABLED", "true").eq_ignore_ascii_case("true");
    validate_origin(&config)?;
    let repositories: Vec<String> = value("AGENT_CONTROL_GITHUB_REPOSITORIES", "")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    let github = if repositories.is_empty() {
        None
    } else {
        Some(connector_github::Config {
            repositories,
            token_file: std::env::var("AGENT_CONTROL_GITHUB_TOKEN_FILE")
                .ok()
                .map(PathBuf::from),
            credential_source: value("AGENT_CONTROL_GITHUB_CREDENTIAL_SOURCE", "file"),
            timeout: std::time::Duration::from_secs(10),
        })
    };
    let files: Vec<String> = value("AGENT_CONTROL_GOOGLE_DRIVE_FILES", "")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    let drive = if files.is_empty() {
        None
    } else {
        Some(connector_google_drive::Config {
            files,
            token_file: std::env::var("AGENT_CONTROL_GOOGLE_DRIVE_TOKEN_FILE")
                .map(PathBuf::from)
                .map_err(|_| Error::new("google_credential_unavailable", 503))?,
            timeout: std::time::Duration::from_secs(10),
        })
    };
    let oauth = match (
        std::env::var("AGENT_CONTROL_GOOGLE_OAUTH_CLIENT_ID").ok(),
        std::env::var("AGENT_CONTROL_GOOGLE_OAUTH_SECRET_FILE").ok(),
    ) {
        (None, None) => None,
        (Some(client_id), Some(secret_file)) => Some(connector_google_drive::oauth::Config {
            client_id,
            secret_file: secret_file.into(),
            origin: config.origin.clone(),
        }),
        _ => return Err(Error::new("invalid_google_oauth_configuration", 400)),
    };
    Ok((config, github, drive, oauth))
}
pub fn validate_origin(c: &Config) -> Result<url::Url> {
    let url = url::Url::parse(&c.origin).map_err(|_| Error::new("invalid_origin", 400))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.scheme() == "http"
            && !matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
        || url.scheme() == "https" && !c.secure_cookies
    {
        return Err(Error::new("invalid_origin", 400));
    }
    Ok(url)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_non_origin_urls() {
        let mut c = Config::local("unused".into());
        for origin in [
            "http://evil.example",
            "ftp://127.0.0.1",
            "https://user:pass@example.com",
            "http://127.0.0.1/path",
            "http://127.0.0.1?secret=x",
            "http://127.0.0.1#fragment",
        ] {
            c.origin = origin.into();
            assert!(validate_origin(&c).is_err());
        }
    }
    #[test]
    fn secure_public_origin_supported() {
        let mut c = Config::local("unused".into());
        c.origin = "https://control.example.com".into();
        assert!(validate_origin(&c).is_err());
        c.secure_cookies = true;
        assert!(validate_origin(&c).is_ok());
    }
}
