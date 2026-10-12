use super::{Manager, config::SCOPE};
use control_core::{
    error::{Error, Result},
    persistence::epoch,
};
use oauth2::{RefreshToken, TokenResponse};
use serde_json::{Value, json};
use zeroize::Zeroizing;
impl Manager {
    pub(crate) fn token_value(
        &self,
        response: &oauth2::basic::BasicTokenResponse,
        old: Option<&Value>,
    ) -> Result<Value> {
        if response.token_type() != &oauth2::basic::BasicTokenType::Bearer {
            return Err(Error::new("google_oauth_response_invalid", 502));
        }
        let scopes = response
            .scopes()
            .map(|s| s.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        if scopes.as_ref().is_some_and(|s| s.as_slice() != [SCOPE])
            || scopes.is_none() && old.is_none()
        {
            return Err(Error::new("google_oauth_scope_rejected", 403));
        }
        let expiry = response
            .expires_in()
            .filter(|d| !d.is_zero() && d.as_secs() <= 86400)
            .ok_or(Error::new("google_oauth_response_invalid", 502))?;
        let access = response.access_token().secret();
        let refresh = response
            .refresh_token()
            .map(|r| r.secret().clone())
            .or_else(|| old.and_then(|v| v["refresh_token"].as_str().map(String::from)))
            .ok_or(Error::new("google_refresh_token_required", 403))?;
        if access.is_empty()
            || access.len() > 4096
            || refresh.is_empty()
            || refresh.len() > 4096
            || access.chars().any(char::is_whitespace)
            || refresh.chars().any(char::is_whitespace)
        {
            return Err(Error::new("google_oauth_response_invalid", 502));
        }
        Ok(
            json!({"client_id":self.config.client_id,"access_token":access,"refresh_token":refresh,"expires_at":epoch()+expiry.as_secs_f64(),"scope":SCOPE,"account":old.map(|v|v["account"].clone()).unwrap_or(json!({}))}),
        )
    }
    pub(crate) async fn token(&self) -> Result<Zeroizing<String>> {
        let _lock = self.lock.lock().await;
        let (state, data, generation, _) = self.store.read("google-drive").await?;
        let data = data
            .filter(|v| {
                state == "connected"
                    && v["client_id"] == self.config.client_id
                    && v["scope"] == SCOPE
            })
            .ok_or(Error::new("google_connection_required", 403))?;
        if data["expires_at"].as_f64().unwrap_or(0.) > epoch() + 60. {
            return Ok(Zeroizing::new(
                data["access_token"]
                    .as_str()
                    .ok_or(Error::new("google_connection_required", 403))?
                    .into(),
            ));
        }
        let refresh = Zeroizing::new(
            data["refresh_token"]
                .as_str()
                .ok_or(Error::new("google_connection_required", 403))?
                .to_owned(),
        );
        let response = self
            .config
            .client(&self.token_endpoint)?
            .exchange_refresh_token(&RefreshToken::new(refresh.to_string()))
            .request_async(&self.http)
            .await;
        let response = match response {
            Ok(r) => r,
            Err(error) => {
                if matches!(error,oauth2::RequestTokenError::ServerResponse(ref e) if e.error()==&oauth2::basic::BasicErrorResponseType::InvalidGrant)
                {
                    self.store.clear("google-drive", "reauth_required").await?;
                    return Err(Error::new("google_reauthentication_required", 403));
                }
                return Err(Error::new("google_refresh_unavailable", 503));
            }
        };
        let updated = match self.token_value(&response, Some(&data)) {
            Ok(v) => v,
            Err(e) => {
                self.store.clear("google-drive", "reauth_required").await?;
                return Err(e);
            }
        };
        self.store
            .refresh("google-drive", generation, &updated)
            .await?;
        Ok(Zeroizing::new(
            updated["access_token"].as_str().unwrap().into(),
        ))
    }
}
