//! Credentials are read only at dispatch, bounded, and never persisted or exposed to agents.
use crate::Config;
use control_core::{
    error::{Error, Result},
    security::files::private_read,
};
use std::{process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt, process::Command};
use zeroize::Zeroizing;
fn unavailable() -> Error {
    Error::new("github_credential_unavailable", 503)
}
fn demo_command() -> Command {
    let mut command = Command::new("gh");
    command.args(["auth", "token", "--hostname", "github.com"]);
    command
}
async fn demo_output(mut command: Command, deadline: Duration) -> Result<Zeroizing<Vec<u8>>> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    tokio::time::timeout(deadline, async move {
        let mut child = command.spawn().map_err(|_| unavailable())?;
        let stdout = child.stdout.take().ok_or_else(unavailable)?;
        let mut data = Zeroizing::new(Vec::new());
        stdout
            .take(4098)
            .read_to_end(&mut data)
            .await
            .map_err(|_| unavailable())?;
        if data.len() > 4097 {
            let _ = child.kill().await;
            return Err(unavailable());
        }
        if !child.wait().await.map_err(|_| unavailable())?.success() {
            return Err(unavailable());
        }
        Ok(data)
    })
    .await
    .map_err(|_| unavailable())?
}
pub async fn token(config: &Config) -> Result<Zeroizing<String>> {
    let raw = if config.credential_source == "gh-demo" {
        demo_output(demo_command(), Duration::from_secs(5)).await?
    } else {
        Zeroizing::new(
            private_read(config.token_file.as_ref().ok_or_else(unavailable)?, 4097)
                .map_err(|_| unavailable())?,
        )
    };
    let value = std::str::from_utf8(&raw).map_err(|_| unavailable())?.trim();
    if value.is_empty() || value.len() > 4096 || value.chars().any(char::is_whitespace) {
        return Err(unavailable());
    }
    Ok(Zeroizing::new(value.to_string()))
}
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn demo_command_is_fixed() {
        let command = demo_command();
        assert_eq!(command.as_std().get_program(), "gh");
        assert_eq!(
            command.as_std().get_args().collect::<Vec<_>>(),
            vec!["auth", "token", "--hostname", "github.com"]
        );
    }
    #[tokio::test]
    async fn demo_output_read_without_real_credentials() {
        let mut c = Command::new("/usr/bin/printf");
        c.args(["%s", "fixture-only-token"]);
        let raw = demo_output(c, Duration::from_secs(1)).await.unwrap();
        assert_eq!(&*raw, b"fixture-only-token");
    }
    #[tokio::test]
    async fn command_errors_do_not_leak() {
        let c = Command::new("/no/such/fixture/gh");
        assert_eq!(
            demo_output(c, Duration::from_secs(1))
                .await
                .unwrap_err()
                .code,
            "github_credential_unavailable"
        );
        let c = Command::new("/usr/bin/false");
        assert_eq!(
            demo_output(c, Duration::from_secs(1))
                .await
                .unwrap_err()
                .code,
            "github_credential_unavailable"
        );
    }
    #[tokio::test]
    async fn demo_output_is_bounded() {
        let mut c = Command::new("/usr/bin/printf");
        c.args(["%s", &"x".repeat(4098)]);
        assert!(demo_output(c, Duration::from_secs(1)).await.is_err());
    }
    #[tokio::test]
    async fn demo_command_timeout_is_bounded() {
        let mut c = Command::new("/bin/sleep");
        c.arg("10");
        assert_eq!(
            demo_output(c, Duration::from_millis(20))
                .await
                .unwrap_err()
                .code,
            "github_credential_unavailable"
        );
    }
}
