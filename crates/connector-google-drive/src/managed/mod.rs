//! Managed Drive execution only. OAuth lifecycle and resource policies live elsewhere.
mod definitions;
mod search;
use crate::oauth::Manager;
use control_core::{
    connections::Management,
    connector::{Connector, Tool},
    error::{Error, Result},
    resources::Metadata,
};
use serde_json::{Value, json};
use std::sync::Arc;
pub struct ManagedDrive {
    pub(crate) manager: Arc<Manager>,
}
impl ManagedDrive {
    pub fn new(manager: Arc<Manager>) -> Self {
        Self { manager }
    }
    pub(crate) fn safe_file(v: &Value) -> Result<Value> {
        let id = v["id"]
            .as_str()
            .filter(|s| crate::native_scope::valid_id(s))
            .ok_or(Error::new("google_response_invalid", 502))?;
        let name = v["name"]
            .as_str()
            .filter(|s| s.chars().count() <= 1024)
            .ok_or(Error::new("google_response_invalid", 502))?;
        let mime = v["mimeType"]
            .as_str()
            .filter(|s| {
                matches!(
                    *s,
                    "application/vnd.google-apps.document"
                        | "text/plain"
                        | "application/vnd.google-apps.folder"
                )
            })
            .ok_or(Error::new("google_file_type_unsupported", 422))?;
        Ok(
            json!({"id":id,"name":name,"mime_type":mime,"url":format!("https://drive.google.com/file/d/{id}/view")}),
        )
    }
}
#[async_trait::async_trait]
impl Connector for ManagedDrive {
    fn id(&self) -> &'static str {
        "google-drive"
    }
    fn tools(&self) -> Vec<Tool> {
        definitions::tools()
    }
    fn management(&self) -> Option<Arc<dyn Management>> {
        Some(self.manager.clone())
    }
    fn requires_resources(&self) -> bool {
        true
    }
    fn validate(&self, tool: &str, p: &Value) -> Result<()> {
        let definition = self
            .tools()
            .into_iter()
            .find(|t| t.name == tool)
            .ok_or(Error::new("google_tool_not_supported", 403))?;
        if ["limit", "max_chars"]
            .iter()
            .any(|key| p.get(*key).is_some_and(|v| v.as_u64().is_none()))
            || !jsonschema::validator_for(&definition.input_schema)
                .map_err(|_| Error::new("invalid_tool_schema", 503))?
                .is_valid(p)
        {
            return Err(Error::new("invalid_tool_parameters", 422));
        }
        Ok(())
    }
    fn preview(&self, tool: &str, p: &Value) -> Option<Value> {
        Some(if tool == "files.read" {
            json!({"operation":tool,"resource_id":p["resource_id"],"file_id":p["file_id"],"max_chars":p["max_chars"].as_u64().unwrap_or(6000)})
        } else {
            json!({"operation":tool,"resource_id":p["resource_id"],"folder_id":p["folder_id"],"query":p["query"].as_str().unwrap_or(""),"limit":p["limit"].as_u64().unwrap_or(10)})
        })
    }
    async fn execute(&self, tool: &str, p: &Value, _: &str) -> Result<Value> {
        self.validate(tool, p)?;
        let root = self
            .manager
            .resources
            .get(self.id(), p["resource_id"].as_str().unwrap())
            .await?;
        let verification = self.manager.verify_native().await?;
        if verification["verified"] != true {
            return Err(Error::new("google_native_access_too_broad", 403));
        }
        if tool == "files.search" {
            return self.search(p, &root.provider_id).await;
        }
        let id = p["file_id"].as_str().unwrap();
        let metadata = self.manager.within(id, &root.provider_id).await?;
        let file = Self::safe_file(&metadata)?;
        let token = self.manager.token().await?;
        let (path, query) = match file["mime_type"].as_str().unwrap() {
            "application/vnd.google-apps.document" => (
                format!("files/{id}/export"),
                vec![("mimeType", "text/plain")],
            ),
            "text/plain" => (
                format!("files/{id}"),
                vec![("alt", "media"), ("supportsAllDrives", "true")],
            ),
            _ => return Err(Error::new("google_file_type_unsupported", 422)),
        };
        let mut header = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", *token))
            .map_err(|_| Error::new("google_credential_unavailable", 503))?;
        header.set_sensitive(true);
        let response = self
            .manager
            .http
            .client
            .get(format!("{}/drive/v3/{path}", self.manager.origin))
            .header(reqwest::header::AUTHORIZATION, header)
            .query(&query)
            .send()
            .await
            .map_err(|_| Error::uncertain("google_request_failed", 502))?;
        let bytes = self.manager.http.bytes(response, 262144).await?;
        let text =
            std::str::from_utf8(&bytes).map_err(|_| Error::new("google_response_invalid", 502))?;
        let max = p["max_chars"].as_u64().unwrap_or(6000) as usize;
        Ok(
            json!({"file":file,"content":text.chars().take(max).collect::<String>(),"truncated":text.chars().count()>max}),
        )
    }
    fn receipt(&self, r: &Value) -> Option<Value> {
        Some(if let Some(files) = r["files"].as_array() {
            json!({"provider":"google-drive","file_count":files.len()})
        } else {
            json!({"provider":"google-drive","character_count":r["content"].as_str().map_or(0,|s|s.chars().count())})
        })
    }
    fn observations(&self, r: &Value) -> Vec<Metadata> {
        let files = if let Some(list) = r["files"].as_array() {
            list.clone()
        } else {
            vec![r["file"].clone()]
        };
        files
            .into_iter()
            .map(|f| Metadata {
                provider_id: f["id"].as_str().unwrap().into(),
                name: f["name"].as_str().unwrap().into(),
                kind: if f["mime_type"] == "application/vnd.google-apps.folder" {
                    "folder"
                } else {
                    "document"
                }
                .into(),
            })
            .collect()
    }
}
