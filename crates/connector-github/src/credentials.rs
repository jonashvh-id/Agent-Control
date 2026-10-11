use crate::Config;
use control_core::{
    error::{Error, Result},
    security::files::private_read,
};
use zeroize::Zeroizing;
pub async fn token(config: &Config) -> Result<Zeroizing<String>> {
    let raw = if config.credential_source == "gh-demo" {
        let mut command = tokio::process::Command::new("gh");
        command
            .args(["auth", "token", "--hostname", "github.com"])
            .kill_on_drop(true);
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), command.output())
            .await
            .map_err(|_| Error::new("github_credential_unavailable", 503))?
            .map_err(|_| Error::new("github_credential_unavailable", 503))?;
        if !result.status.success() {
            return Err(Error::new("github_credential_unavailable", 503));
        }
        Zeroizing::new(result.stdout)
    } else {
        Zeroizing::new(
            private_read(
                config
                    .token_file
                    .as_ref()
                    .ok_or(Error::new("github_credential_unavailable", 503))?,
                4097,
            )
            .map_err(|_| Error::new("github_credential_unavailable", 503))?,
        )
    };
    let value = std::str::from_utf8(&raw)
        .map_err(|_| Error::new("github_credential_unavailable", 503))?
        .trim();
    if value.is_empty() || value.len() > 4096 || value.chars().any(char::is_whitespace) {
        return Err(Error::new("github_credential_unavailable", 503));
    }
    Ok(Zeroizing::new(value.to_string()))
}
