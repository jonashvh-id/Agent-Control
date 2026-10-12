use crate::{Config, credentials, definitions};
use control_core::{
    connector::{Connector, Tool},
    error::{Error, Result},
};
use reqwest::{
    Client,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use serde_json::{Value, json};
pub struct Drive {
    config: Config,
    client: Client,
    origin: String,
}
impl Drive {
    pub fn new(config: Config) -> Result<Self> {
        config.validate()?;
        let client = Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(config.timeout)
            .build()
            .map_err(|_| Error::new("google_client_unavailable", 503))?;
        Ok(Self {
            config,
            client,
            origin: "https://www.googleapis.com".into(),
        })
    }
    async fn get(&self, path: &str, query: &[(&str, &str)]) -> Result<Vec<u8>> {
        let token = credentials::token(&self.config)?;
        let mut auth = HeaderValue::from_str(&format!("Bearer {}", *token))
            .map_err(|_| Error::new("google_credential_unavailable", 503))?;
        auth.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, auth);
        let mut response = self
            .client
            .get(format!("{}/drive/v3/{path}", self.origin))
            .headers(headers)
            .query(query)
            .send()
            .await
            .map_err(|_| Error::uncertain("google_request_failed", 502))?;
        match response.status().as_u16() {
            200 => (),
            401 | 403 | 404 => return Err(Error::new("google_access_denied", 403)),
            429 => return Err(Error::new("google_rate_limited", 429)),
            _ => return Err(Error::uncertain("google_request_failed", 502)),
        }
        let mut data = vec![];
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| Error::uncertain("google_response_invalid", 502))?
        {
            if data.len() + chunk.len() > 262144 {
                return Err(Error::uncertain("google_result_too_large", 502));
            }
            data.extend_from_slice(&chunk);
        }
        Ok(data)
    }
    async fn metadata(&self, id: &str) -> Result<Value> {
        if !self.config.permits(id) {
            return Err(Error::new("google_file_denied", 403));
        }
        let raw = self
            .get(
                &format!("files/{id}"),
                &[
                    ("fields", "id,name,mimeType,trashed"),
                    ("supportsAllDrives", "true"),
                ],
            )
            .await?;
        let v: Value = serde_json::from_slice(&raw)
            .map_err(|_| Error::uncertain("google_response_invalid", 502))?;
        let name = v["name"]
            .as_str()
            .filter(|s| s.chars().count() <= 1024)
            .ok_or(Error::uncertain("google_response_invalid", 502))?;
        if v["id"] != id || v["trashed"] != false {
            return Err(Error::new("google_file_unavailable", 403));
        }
        let mime = v["mimeType"]
            .as_str()
            .filter(|s| matches!(*s, "application/vnd.google-apps.document" | "text/plain"))
            .ok_or(Error::new("google_file_type_unsupported", 422))?;
        Ok(
            json!({"id":id,"name":name,"mime_type":mime,"url":format!("https://drive.google.com/file/d/{id}/view")}),
        )
    }
}
#[async_trait::async_trait]
impl Connector for Drive {
    fn id(&self) -> &'static str {
        "google-drive"
    }
    fn tools(&self) -> Vec<Tool> {
        definitions::tools(&self.config)
    }
    fn validate(&self, tool: &str, p: &Value) -> Result<()> {
        let schema = self
            .tools()
            .into_iter()
            .find(|t| t.name == tool)
            .ok_or(Error::new("google_tool_not_supported", 403))?;
        if tool == "files.read"
            && !p["file_id"]
                .as_str()
                .is_some_and(|id| self.config.permits(id))
        {
            return Err(Error::new("google_file_denied", 403));
        }
        if ["limit", "max_chars"]
            .iter()
            .any(|key| p.get(*key).is_some_and(|v| v.as_u64().is_none()))
            || !jsonschema::validator_for(&schema.input_schema)
                .map_err(|_| Error::new("invalid_tool_schema", 503))?
                .is_valid(p)
        {
            return Err(Error::new("invalid_tool_parameters", 422));
        }
        Ok(())
    }
    fn preview(&self, tool: &str, p: &Value) -> Option<Value> {
        Some(if tool == "files.read" {
            json!({"operation":tool,"file_id":p["file_id"],"max_chars":p["max_chars"].as_u64().unwrap_or(6000)})
        } else {
            json!({"operation":tool,"query":p["query"].as_str().unwrap_or(""),"limit":p["limit"].as_u64().unwrap_or(10),"permitted_files":self.config.files})
        })
    }
    async fn execute(&self, tool: &str, p: &Value, _: &str) -> Result<Value> {
        self.validate(tool, p)?;
        if tool == "files.search" {
            let query = p["query"].as_str().unwrap_or("").to_lowercase();
            let limit = p["limit"].as_u64().unwrap_or(10) as usize;
            let mut files = vec![];
            for id in &self.config.files {
                let file = self.metadata(id).await?;
                if file["name"]
                    .as_str()
                    .unwrap()
                    .to_lowercase()
                    .contains(&query)
                {
                    files.push(file);
                }
                if files.len() == limit {
                    break;
                }
            }
            return Ok(json!({"files":files}));
        }
        let id = p["file_id"].as_str().unwrap();
        let file = self.metadata(id).await?;
        let raw = if file["mime_type"] == "application/vnd.google-apps.document" {
            self.get(&format!("files/{id}/export"), &[("mimeType", "text/plain")])
                .await?
        } else {
            self.get(
                &format!("files/{id}"),
                &[("alt", "media"), ("supportsAllDrives", "true")],
            )
            .await?
        };
        let text = std::str::from_utf8(&raw)
            .map_err(|_| Error::uncertain("google_response_invalid", 502))?;
        let max = p["max_chars"].as_u64().unwrap_or(6000) as usize;
        let content: String = text.chars().take(max).collect();
        Ok(json!({"file":file,"content":content,"truncated":text.chars().count()>max}))
    }
    fn receipt(&self, result: &Value) -> Option<Value> {
        Some(if let Some(files) = result["files"].as_array() {
            json!({"provider":"google-drive","file_count":files.len()})
        } else {
            json!({"provider":"google-drive","character_count":result["content"].as_str().map_or(0,|s|s.chars().count())})
        })
    }
}
#[cfg(test)]
mod tests;
