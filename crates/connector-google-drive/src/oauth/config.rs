use control_core::{
    error::{Error, Result},
    security::files::private_read,
};
use oauth2::{
    AuthType, AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet, RedirectUrl, TokenUrl,
    basic::BasicClient,
};
use std::path::PathBuf;
use zeroize::Zeroizing;
pub const SCOPE: &str = "https://www.googleapis.com/auth/drive.readonly";
pub type Client =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;
#[derive(Clone)]
pub struct Config {
    pub client_id: String,
    pub secret_file: PathBuf,
    pub origin: String,
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        let url = url::Url::parse(&self.origin)
            .map_err(|_| Error::new("invalid_google_oauth_configuration", 400))?;
        if !self.client_id.ends_with(".apps.googleusercontent.com")
            || self.client_id.len() > 256
            || self
                .client_id
                .chars()
                .any(|c| !c.is_ascii_alphanumeric() && !matches!(c, '.' | '-' | '_'))
            || self.secret_file.as_os_str().is_empty()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
            || !(url.scheme() == "https" && url.host_str().is_some()
                || url.scheme() == "http"
                    && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]")))
        {
            return Err(Error::new("invalid_google_oauth_configuration", 400));
        }
        Ok(())
    }
    pub fn redirect_uri(&self) -> String {
        format!(
            "{}/api/v1/connections/google-drive/callback",
            self.origin.trim_end_matches('/')
        )
    }
    pub fn client(&self, token_endpoint: &str) -> Result<Client> {
        self.validate()?;
        let raw = Zeroizing::new(
            private_read(&self.secret_file, 4097)
                .map_err(|_| Error::new("google_oauth_client_unavailable", 503))?,
        );
        let secret = std::str::from_utf8(&raw)
            .map_err(|_| Error::new("google_oauth_client_unavailable", 503))?
            .trim();
        if secret.is_empty() || secret.len() > 4096 || secret.chars().any(char::is_whitespace) {
            return Err(Error::new("google_oauth_client_unavailable", 503));
        }
        Ok(BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_client_secret(ClientSecret::new(secret.into()))
            .set_auth_uri(
                AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".into()).unwrap(),
            )
            .set_token_uri(
                TokenUrl::new(token_endpoint.into())
                    .map_err(|_| Error::new("google_oauth_client_unavailable", 503))?,
            )
            .set_auth_type(AuthType::RequestBody)
            .set_redirect_uri(RedirectUrl::new(self.redirect_uri()).unwrap()))
    }
}
