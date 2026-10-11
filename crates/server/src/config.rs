use control_core::{
    error::{Error, Result},
    models::Config,
};
use std::path::PathBuf;
pub fn from_env() -> Result<(Config, Option<connector_github::Config>)> {
    let value = |key: &str, default: &str| std::env::var(key).unwrap_or(default.into());
    let mut config = Config::local(PathBuf::from(value("AGENT_CONTROL_DATA_DIR", "var-rust")));
    config.origin = value("AGENT_CONTROL_ORIGIN", "http://127.0.0.1:8010")
        .trim_end_matches('/')
        .into();
    config.secure_cookies = value("AGENT_CONTROL_SECURE_COOKIES", "false") == "true";
    config.execution_enabled = value("AGENT_CONTROL_EXECUTION_ENABLED", "false") == "true";
    config.mcp_enabled = value("AGENT_CONTROL_MCP_ENABLED", "true") == "true";
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
    Ok((config, github))
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
