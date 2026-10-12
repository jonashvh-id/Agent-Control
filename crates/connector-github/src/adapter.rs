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
pub struct GitHub {
    config: Config,
    client: Client,
    origin: String,
}
impl GitHub {
    pub fn new(config: Config, loopback: bool) -> Result<Self> {
        config.validate(loopback)?;
        let client = Client::builder()
            .timeout(config.timeout)
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .no_proxy()
            .https_only(true)
            .build()
            .map_err(|_| Error::new("github_client_unavailable", 503))?;
        Ok(Self {
            config,
            client,
            origin: "https://api.github.com".into(),
        })
    }
    fn issue(&self, value: &Value, repo: &str) -> Result<Value> {
        let number = value["number"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or(Error::uncertain("github_response_invalid", 502))?;
        let title = value["title"]
            .as_str()
            .filter(|s| s.chars().count() <= 1024)
            .ok_or(Error::uncertain("github_response_invalid", 502))?;
        let raw = value["html_url"]
            .as_str()
            .ok_or(Error::uncertain("github_response_invalid", 502))?;
        let url =
            url::Url::parse(raw).map_err(|_| Error::uncertain("github_response_invalid", 502))?;
        if raw.len() > 2048
            || url.scheme() != "https"
            || url.host_str() != Some("github.com")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || !url
                .path()
                .eq_ignore_ascii_case(&format!("/{repo}/issues/{number}"))
            || !matches!(value["state"].as_str(), Some("open" | "closed"))
        {
            return Err(Error::uncertain("github_response_invalid", 502));
        }
        Ok(json!({"number":number,"title":title,"url":raw,"state":value["state"]}))
    }
}
#[async_trait::async_trait]
impl Connector for GitHub {
    fn id(&self) -> &'static str {
        "github"
    }
    fn tools(&self) -> Vec<Tool> {
        definitions::tools(&self.config)
    }
    fn validate(&self, tool: &str, p: &Value) -> Result<()> {
        if !matches!(tool, "issues.list" | "issues.create") {
            return Err(Error::new("github_tool_not_supported", 403));
        }
        if !p["repository"]
            .as_str()
            .is_some_and(|r| self.config.permits(r))
        {
            return Err(Error::new("github_repository_denied", 403));
        }
        if tool == "issues.list"
            && ["limit", "page"]
                .iter()
                .any(|key| p.get(*key).is_some_and(|v| v.as_u64().is_none()))
        {
            return Err(Error::new("invalid_tool_parameters", 422));
        }
        Ok(())
    }
    fn preview(&self, tool: &str, p: &Value) -> Option<Value> {
        if tool == "issues.list" {
            return Some(json!({"operation":tool,"repository":p["repository"],
                "state":p["state"].as_str().unwrap_or("open"),
                "limit":p["limit"].as_u64().unwrap_or(10),
                "page":p["page"].as_u64().unwrap_or(1)}));
        }
        Some(
            json!({"repository":p["repository"],"title":p["title"].as_str().unwrap_or(""),"body":p["body"].as_str().unwrap_or(""),"operation":tool}),
        )
    }
    async fn execute(&self, tool: &str, p: &Value, _execution_id: &str) -> Result<Value> {
        self.validate(tool, p)?;
        let schema = self
            .tools()
            .into_iter()
            .find(|t| t.name == tool)
            .ok_or(Error::new("github_tool_not_supported", 403))?;
        if !jsonschema::validator_for(&schema.input_schema)
            .map_err(|_| Error::new("invalid_tool_schema", 503))?
            .is_valid(p)
        {
            return Err(Error::new("invalid_tool_parameters", 422));
        }
        let repo = p["repository"]
            .as_str()
            .ok_or(Error::new("github_repository_denied", 403))?;
        let mut endpoint = url::Url::parse(&self.origin)
            .map_err(|_| Error::new("github_client_unavailable", 503))?;
        endpoint
            .path_segments_mut()
            .map_err(|_| Error::new("github_client_unavailable", 503))?
            .extend([
                "repos",
                repo.split('/').next().unwrap(),
                repo.split('/').nth(1).unwrap(),
                "issues",
            ]);
        let token = credentials::token(&self.config).await?;
        let mut headers = HeaderMap::new();
        let mut authorization = HeaderValue::from_str(&format!("Bearer {}", *token))
            .map_err(|_| Error::new("github_credential_unavailable", 503))?;
        authorization.set_sensitive(true);
        headers.insert(AUTHORIZATION, authorization);
        headers.insert(
            "accept",
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "x-github-api-version",
            HeaderValue::from_static("2026-03-10"),
        );
        headers.insert("user-agent", HeaderValue::from_static("Agent-Control/0.3"));
        let create = tool == "issues.create";
        let request = if create {
            self.client
                .post(endpoint)
                .json(&json!({"title":p["title"],"body":p["body"]}))
        } else {
            self.client.get(endpoint).query(&[
                ("state", p["state"].as_str().unwrap_or("open").to_string()),
                ("per_page", p["limit"].as_u64().unwrap_or(10).to_string()),
                ("page", p["page"].as_u64().unwrap_or(1).to_string()),
            ])
        };
        let mut response = request.headers(headers).send().await.map_err(|e| {
            if e.is_timeout() {
                Error::uncertain("github_timeout", 504)
            } else {
                Error::uncertain("github_response_invalid", 502)
            }
        })?;
        let status = response.status().as_u16();
        if matches!(status, 401 | 403 | 404) {
            return Err(Error::new("github_access_denied", 403));
        }
        if status == 429 {
            return Err(Error::new("github_rate_limited", 429));
        }
        if status != if create { 201 } else { 200 } {
            return Err(Error::uncertain("github_request_failed", 502));
        }
        let mut raw = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| {
            if e.is_timeout() {
                Error::uncertain("github_timeout", 504)
            } else {
                Error::uncertain("github_response_invalid", 502)
            }
        })? {
            if raw.len() + chunk.len() > 262144 {
                return Err(Error::uncertain("github_result_too_large", 502));
            }
            raw.extend_from_slice(&chunk);
        }
        let payload: Value = serde_json::from_slice(&raw)
            .map_err(|_| Error::uncertain("github_response_invalid", 502))?;
        if create {
            Ok(json!({"issue":self.issue(&payload,repo)?}))
        } else {
            let list = payload
                .as_array()
                .ok_or(Error::uncertain("github_response_invalid", 502))?;
            let mut issues = vec![];
            for item in list
                .iter()
                .filter(|i| i.is_object() && i.get("pull_request").is_none())
                .take(p["limit"].as_u64().unwrap_or(10) as usize)
            {
                issues.push(self.issue(item, repo)?);
            }
            Ok(json!({"issues":issues}))
        }
    }
    fn receipt(&self, result: &Value) -> Option<Value> {
        Some(if result.get("issue").is_some() {
            json!({"provider":"github","issue_number":result["issue"]["number"],"issue_url":result["issue"]["url"]})
        } else {
            json!({"provider":"github","issue_count":result["issues"].as_array().map_or(0,|a|a.len())})
        })
    }
}
#[cfg(test)]
mod tests;
