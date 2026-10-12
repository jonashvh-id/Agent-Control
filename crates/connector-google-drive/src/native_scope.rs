use crate::oauth::Manager;
use control_core::{
    error::{Error, Result},
    persistence::epoch,
};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
pub(crate) fn valid_id(id: &str) -> bool {
    static IDENTIFIER: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"^[A-Za-z0-9_-]{10,128}$").unwrap());
    IDENTIFIER.is_match(id)
}
fn contained(
    id: &str,
    roots: &HashSet<String>,
    items: &HashMap<String, Vec<String>>,
    seen: &mut HashSet<String>,
    depth: usize,
) -> bool {
    if roots.contains(id) {
        return true;
    }
    if depth >= 20 || !seen.insert(id.into()) {
        return false;
    }
    items.get(id).is_some_and(|parents| {
        parents
            .iter()
            .any(|p| contained(p, roots, items, seen, depth + 1))
    })
}
impl Manager {
    pub(crate) async fn metadata(&self, id: &str) -> Result<Value> {
        if !crate::native_scope::valid_id(id) {
            return Err(Error::new("invalid_google_resource", 422));
        }
        let token = self.token().await?;
        let mut v = self
            .http
            .json(
                &format!("{}/drive/v3/files/{id}", self.origin),
                &token,
                &[
                    (
                        "fields",
                        "id,name,mimeType,parents,trashed,capabilities(canEdit)",
                    ),
                    ("supportsAllDrives", "true"),
                ],
            )
            .await?;
        if v.get("parents").is_none() {
            v["parents"] = json!([]);
        }
        if v["id"] != id
            || v["trashed"] != false
            || v["name"].as_str().is_none_or(|s| s.chars().count() > 1024)
            || v["parents"].as_array().is_none_or(|a| {
                a.len() > 10
                    || a.iter()
                        .any(|p| p.as_str().is_none_or(|s| !crate::native_scope::valid_id(s)))
            })
        {
            return Err(Error::new("google_response_invalid", 502));
        }
        Ok(v)
    }
    pub(crate) async fn verify_native(&self) -> Result<Value> {
        let _lock = self.extent_lock.lock().await;
        let (generation, revision) = self.store.revision("google-drive").await?;
        let result = self.inspect_extent().await;
        let report = match &result {
            Ok(v) => v.clone(),
            Err(e) => json!({"verified":false,"checked_at":epoch(),"reason":e.code}),
        };
        self.store
            .verification_checked("google-drive", &report, generation, revision)
            .await?;
        result
    }
    async fn inspect_extent(&self) -> Result<Value> {
        let roots = self
            .resources
            .list("google-drive")
            .await?
            .into_iter()
            .filter(|r| r["enabled"] == true)
            .collect::<Vec<_>>();
        if roots.is_empty() {
            return Err(Error::new("google_folders_required", 403));
        }
        let mut ids = HashSet::new();
        for root in &roots {
            let id = root["provider_id"]
                .as_str()
                .ok_or(Error::new("private_store_unavailable", 503))?;
            let metadata = self.metadata(id).await?;
            if metadata["mimeType"] != "application/vnd.google-apps.folder"
                || metadata["capabilities"]["canEdit"] != false
            {
                return Err(Error::new("google_reader_folder_required", 403));
            }
            ids.insert(id.to_owned());
        }
        let token = self.token().await?;
        let mut page = String::new();
        let mut items = HashMap::new();
        let mut total = 0;
        for _ in 0..10 {
            let value = self
                .http
                .json(
                    &format!("{}/drive/v3/files", self.origin),
                    &token,
                    &[
                        ("q", "trashed = false"),
                        ("spaces", "drive"),
                        ("corpora", "user"),
                        ("supportsAllDrives", "true"),
                        ("includeItemsFromAllDrives", "true"),
                        ("pageSize", "100"),
                        ("pageToken", &page),
                        ("fields", "nextPageToken,incompleteSearch,files(id,parents)"),
                    ],
                )
                .await?;
            if value["incompleteSearch"] == true {
                return Err(Error::new("google_native_extent_incomplete", 403));
            }
            let files = value["files"]
                .as_array()
                .filter(|v| v.len() <= 100)
                .ok_or(Error::new("google_response_invalid", 502))?;
            total += files.len();
            if total > 1000 {
                return Err(Error::new("google_native_extent_too_large", 403));
            }
            for file in files {
                let id = file["id"]
                    .as_str()
                    .filter(|s| crate::native_scope::valid_id(s))
                    .ok_or(Error::new("google_response_invalid", 502))?;
                let parents = match file.get("parents") {
                    None => vec![],
                    Some(v) => v
                        .as_array()
                        .filter(|a| a.len() <= 10)
                        .ok_or(Error::new("google_response_invalid", 502))?
                        .iter()
                        .map(|p| {
                            p.as_str()
                                .filter(|s| crate::native_scope::valid_id(s))
                                .map(String::from)
                                .ok_or(Error::new("google_response_invalid", 502))
                        })
                        .collect::<Result<Vec<_>>>()?,
                };
                if items.insert(id.to_owned(), parents).is_some() {
                    return Err(Error::new("google_response_invalid", 502));
                }
            }
            page = value["nextPageToken"].as_str().unwrap_or("").to_owned();
            if page.is_empty() {
                break;
            }
            if page.len() > 2048 {
                return Err(Error::new("google_response_invalid", 502));
            }
        }
        if !page.is_empty() {
            return Err(Error::new("google_native_extent_too_large", 403));
        }
        let outside = items
            .keys()
            .filter(|id| !contained(id, &ids, &items, &mut HashSet::new(), 0))
            .count();
        if !ids.iter().all(|r| items.contains_key(r)) {
            return Err(Error::new("google_native_extent_incomplete", 403));
        }
        Ok(
            json!({"verified":outside==0,"checked_at":epoch(),"visible_objects":total,"selected_folders":roots.len(),"outside_objects":outside,"reason":if outside==0{"native_inventory_checked"}else{"google_native_access_too_broad"},"guarantee":"Snapshot of listed accessible objects and reader folder roles; not an OAuth folder-scoped grant. Every dispatch checks again. Native ACLs must remain restricted."}),
        )
    }
    pub(crate) async fn within(&self, id: &str, root: &str) -> Result<Value> {
        let boundaries = self
            .resources
            .list("google-drive")
            .await?
            .into_iter()
            .filter_map(|r| r["provider_id"].as_str().map(String::from))
            .filter(|id| id != root)
            .collect::<HashSet<_>>();
        let first = self.metadata(id).await?;
        let mut current = first.clone();
        let mut seen = HashSet::new();
        for _ in 0..20 {
            let current_id = current["id"].as_str().unwrap();
            if boundaries.contains(current_id) {
                return Err(Error::new("google_use_specific_resource", 403));
            }
            if current_id == root {
                return Ok(first);
            }
            if !seen.insert(current_id.to_owned()) {
                break;
            }
            let parents = current["parents"].as_array().unwrap();
            if parents.len() != 1 {
                break;
            }
            let parent = parents[0].as_str().unwrap();
            if boundaries.contains(parent) {
                return Err(Error::new("google_use_specific_resource", 403));
            }
            if parent == root {
                return Ok(first);
            }
            current = self.metadata(parent).await?;
            if current["mimeType"] != "application/vnd.google-apps.folder" {
                break;
            }
        }
        Err(Error::new("google_file_outside_resource", 403))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn membership_never_follows_a_cycle_or_unselected_root() {
        let roots = HashSet::from(["a".into()]);
        let items = HashMap::from([
            ("b".into(), vec!["a".into()]),
            ("x".into(), vec!["y".into()]),
            ("y".into(), vec!["x".into()]),
        ]);
        assert!(contained("b", &roots, &items, &mut HashSet::new(), 0));
        assert!(!contained("x", &roots, &items, &mut HashSet::new(), 0));
        assert!(!contained(
            "outside",
            &roots,
            &items,
            &mut HashSet::new(),
            0
        ));
    }
}
