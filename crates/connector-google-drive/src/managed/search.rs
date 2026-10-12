use super::*;
impl ManagedDrive {
    pub(crate) async fn search(&self, p: &Value, root: &str) -> Result<Value> {
        let folder = p["folder_id"].as_str().unwrap_or(root);
        let metadata = self.manager.within(folder, root).await?;
        if metadata["mimeType"] != "application/vnd.google-apps.folder" {
            return Err(Error::new("google_reader_folder_required", 403));
        }
        let token = self.manager.token().await?;
        let q = format!("'{folder}' in parents and trashed = false");
        let wanted = p["query"].as_str().unwrap_or("").to_lowercase();
        let limit = p["limit"].as_u64().unwrap_or(10) as usize;
        let mut page = String::new();
        let mut files = vec![];
        let boundaries = self
            .manager
            .resources
            .list("google-drive")
            .await?
            .into_iter()
            .filter_map(|r| r["provider_id"].as_str().map(String::from))
            .filter(|id| id != root)
            .collect::<std::collections::HashSet<_>>();
        for _ in 0..5 {
            let v=self.manager.http.json(&format!("{}/drive/v3/files",self.manager.origin),&token,&[("q",&q),("pageSize","100"),("pageToken",&page),("supportsAllDrives","true"),("includeItemsFromAllDrives","true"),("fields","nextPageToken,incompleteSearch,files(id,name,mimeType,parents,trashed)")]).await?;
            if v["incompleteSearch"] == true {
                return Err(Error::new("google_native_extent_incomplete", 403));
            }
            let entries = v["files"]
                .as_array()
                .filter(|a| a.len() <= 100)
                .ok_or(Error::new("google_response_invalid", 502))?;
            for file in entries {
                if file["id"]
                    .as_str()
                    .is_some_and(|id| boundaries.contains(id))
                {
                    continue;
                }
                if file["trashed"] != false
                    || !file["parents"]
                        .as_array()
                        .is_some_and(|a| a.len() == 1 && a[0] == folder)
                {
                    return Err(Error::new("google_response_invalid", 502));
                }
                if !matches!(
                    file["mimeType"].as_str(),
                    Some(
                        "application/vnd.google-apps.document"
                            | "application/vnd.google-apps.folder"
                            | "text/plain"
                    )
                ) {
                    continue;
                }
                let safe = Self::safe_file(file)?;
                if safe["name"]
                    .as_str()
                    .unwrap()
                    .to_lowercase()
                    .contains(&wanted)
                {
                    files.push(safe);
                }
                if files.len() == limit {
                    return Ok(json!({"files":files}));
                }
            }
            page = v["nextPageToken"].as_str().unwrap_or("").into();
            if page.is_empty() {
                return Ok(json!({"files":files}));
            }
            if page.len() > 2048 {
                return Err(Error::new("google_response_invalid", 502));
            }
        }
        Err(Error::new("google_folder_scan_limit", 422))
    }
}
