use super::{Manager, config::SCOPE};
use control_core::{
    connections::{Callback, Context, Management, Start},
    error::{Error, Result},
    persistence::epoch,
    resources::Metadata,
};
use oauth2::{AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, Scope};
use serde_json::{Value, json};
#[async_trait::async_trait]
impl Management for Manager {
    async fn status(&self) -> Result<Value> {
        let (state, data, _, verification) = self.store.read("google-drive").await?;
        Ok(
            json!({"mode":"dedicated_reader_oauth","state":state,"account":data.as_ref().map(|v|v["account"].clone()).unwrap_or(json!({})),"expires_at":data.as_ref().map(|v|v["expires_at"].clone()),"automatic_refresh":true,"native_verification":verification,"scope":SCOPE,"redirect_uri":self.config.redirect_uri(),"native_boundary":"The dedicated Google identity must have reader access only to the selected folders. OAuth itself does not restrict this scope to folders."}),
        )
    }
    async fn begin(&self, context: Context) -> Result<Start> {
        let _lock = self.lock.lock().await;
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let (url, state) = self
            .config
            .client(&self.token_endpoint)?
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new(SCOPE.into()))
            .set_pkce_challenge(challenge)
            .add_extra_param("access_type", "offline")
            .add_extra_param("prompt", "consent select_account")
            .add_extra_param("include_granted_scopes", "false")
            .url();
        let hash = self.store.keys.hash(state.secret());
        let cipher = self.store.keys.seal_private(
            &format!("oauth-attempt:{hash}"),
            &json!({"verifier":verifier.secret(),"client_id":self.config.client_id}),
        )?;
        let mut db = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query("DELETE FROM oauth_attempts WHERE expires_at<=? OR consumed=1 OR (session_hash=? AND connector='google-drive')").bind(epoch()).bind(&context.session_hash).execute(&mut *db).await?;
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM oauth_attempts")
            .fetch_one(&mut *db)
            .await?;
        if count >= 100 {
            return Err(Error::new("oauth_capacity_reached", 429));
        }
        sqlx::query("INSERT INTO oauth_attempts VALUES(?,'google-drive',?,?,?,?,?,0)")
            .bind(hash)
            .bind(context.human_id)
            .bind(context.session_hash)
            .bind(context.cookie_hash)
            .bind(cipher)
            .bind(epoch() + 600.)
            .execute(&mut *db)
            .await?;
        db.commit().await?;
        Ok(Start {
            authorization_url: url.to_string(),
        })
    }
    async fn finish(&self, cookie_hash: String, response: Callback) -> Result<String> {
        if response.state.len() > 128
            || response
                .issuer
                .as_deref()
                .is_some_and(|s| s != "https://accounts.google.com")
            || response.code.as_ref().is_some_and(|c| c.len() > 2048)
        {
            return Err(Error::new("oauth_state_rejected", 403));
        }
        self.store
            .pending("google-drive", &response.state, &cookie_hash)
            .await?;
        let _lock = self.lock.lock().await;
        let data = self
            .store
            .consume("google-drive", &response.state, &cookie_hash)
            .await?;
        if response.error.is_some() {
            return Err(Error::new("google_oauth_denied", 403));
        }
        if data["client_id"] != self.config.client_id {
            return Err(Error::new("oauth_state_rejected", 403));
        }
        let code = response
            .code
            .filter(|s| !s.is_empty())
            .ok_or(Error::new("google_oauth_response_invalid", 400))?;
        let verifier = data["verifier"]
            .as_str()
            .ok_or(Error::new("private_store_unavailable", 503))?;
        let token = self
            .config
            .client(&self.token_endpoint)?
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(PkceCodeVerifier::new(verifier.into()))
            .request_async(&self.http)
            .await
            .map_err(|_| Error::new("google_oauth_exchange_failed", 502))?;
        let mut value = self.token_value(&token, None)?;
        let identity = self
            .http
            .json(
                &format!("{}/drive/v3/about", self.origin),
                value["access_token"].as_str().unwrap(),
                &[("fields", "user(displayName,emailAddress)")],
            )
            .await?;
        let account = &identity["user"];
        if account["emailAddress"]
            .as_str()
            .is_none_or(|s| s.len() > 320)
            || account["displayName"]
                .as_str()
                .is_none_or(|s| s.chars().count() > 200)
        {
            return Err(Error::new("google_response_invalid", 502));
        }
        value["account"] = json!({"name":account["displayName"],"email":account["emailAddress"]});
        let human = data["human_id"]
            .as_str()
            .ok_or(Error::new("private_store_unavailable", 503))?;
        self.store
            .replace(
                "google-drive",
                &value,
                human,
                data["session_hash"].as_str().unwrap(),
            )
            .await?;
        Ok(human.into())
    }
    async fn disconnect(&self) -> Result<Value> {
        let _lock = self.lock.lock().await;
        let (_, data, _, _) = self.store.read("google-drive").await?;
        self.store.clear("google-drive", "disconnected").await?;
        let confirmed =
            if let Some(token) = data.and_then(|v| v["refresh_token"].as_str().map(String::from)) {
                self.http
                    .client
                    .post(&self.revoke_endpoint)
                    .form(&[("token", token)])
                    .send()
                    .await
                    .is_ok_and(|r| r.status().as_u16() == 200)
            } else {
                true
            };
        Ok(json!({"state":"disconnected","native_revocation_confirmed":confirmed}))
    }
    async fn inspect_resource(&self, id: &str) -> Result<Metadata> {
        let value = self.metadata(id).await?;
        if value["mimeType"] != "application/vnd.google-apps.folder"
            || value["capabilities"]["canEdit"] != false
        {
            return Err(Error::new("google_reader_folder_required", 403));
        }
        Ok(Metadata {
            provider_id: id.into(),
            name: value["name"].as_str().unwrap().into(),
            kind: "folder".into(),
        })
    }
    async fn verify_extent(&self) -> Result<Value> {
        self.verify_native().await
    }
    async fn invalidate_extent(&self) -> Result<()> {
        self.store.verification("google-drive", &json!({})).await
    }
    async fn candidates(&self) -> Result<Value> {
        let token = self.token().await?;
        let value = self
            .http
            .json(
                &format!("{}/drive/v3/files", self.origin),
                &token,
                &[
                    (
                        "q",
                        "mimeType = 'application/vnd.google-apps.folder' and trashed = false",
                    ),
                    ("pageSize", "100"),
                    ("supportsAllDrives", "true"),
                    ("includeItemsFromAllDrives", "true"),
                    (
                        "fields",
                        "nextPageToken,incompleteSearch,files(id,name,capabilities(canEdit))",
                    ),
                ],
            )
            .await?;
        if value["incompleteSearch"] == true {
            return Err(Error::new("google_native_extent_incomplete", 403));
        }
        let files = value["files"]
            .as_array()
            .filter(|a| a.len() <= 100)
            .ok_or(Error::new("google_response_invalid", 502))?;
        let mut folders = vec![];
        for file in files {
            let id = file["id"]
                .as_str()
                .filter(|s| crate::native_scope::valid_id(s))
                .ok_or(Error::new("google_response_invalid", 502))?;
            let name = file["name"]
                .as_str()
                .filter(|s| s.chars().count() <= 1024)
                .ok_or(Error::new("google_response_invalid", 502))?;
            folders.push(json!({"provider_id":id,"name":name,"reader_only":file["capabilities"]["canEdit"]==false}));
        }
        Ok(
            json!({"folders":folders,"more_available":value["nextPageToken"].as_str().is_some_and(|s|!s.is_empty())}),
        )
    }
}
