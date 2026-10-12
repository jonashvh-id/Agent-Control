use super::*;
pub const FILE: &str = "fixture_document_001";
pub struct DriveProvider {
    pub calls: AtomicUsize,
}
#[async_trait::async_trait]
impl Connector for DriveProvider {
    fn id(&self) -> &'static str {
        "google-drive"
    }
    fn tools(&self) -> Vec<Tool> {
        connector_google_drive::definitions::tools(&connector_google_drive::Config {
            files: vec![FILE.into()],
            token_file: "unused-test-file".into(),
            timeout: std::time::Duration::from_secs(1),
        })
    }
    fn validate(&self, tool: &str, p: &Value) -> Result<()> {
        if tool == "files.read" && p["file_id"] != FILE {
            return Err(Error::new("google_file_denied", 403));
        }
        Ok(())
    }
    fn preview(&self, tool: &str, p: &Value) -> Option<Value> {
        Some(json!({"operation":tool,"parameters":p}))
    }
    async fn execute(&self, tool: &str, p: &Value, _: &str) -> Result<Value> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let file = json!({"id":FILE,"name":"Isolated pilot document","mime_type":"text/plain","url":format!("https://drive.google.com/file/d/{FILE}/view")});
        Ok(if tool == "files.search" {
            json!({"files":[file]})
        } else {
            let text = "This is an isolated provider fixture for the Codex pilot, not live Google Drive content.";
            let max = p["max_chars"].as_u64().unwrap_or(6000) as usize;
            json!({"file":file,"content":text.chars().take(max).collect::<String>(),"truncated":text.chars().count()>max})
        })
    }
    fn receipt(&self, _: &Value) -> Option<Value> {
        Some(json!({"provider":"google-drive","fixture":true}))
    }
}
