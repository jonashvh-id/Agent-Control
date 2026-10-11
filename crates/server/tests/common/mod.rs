#![allow(dead_code)]
use control_core::{
    App,
    connector::{Connector, Tool},
    error::{Error, Result},
    models::{Config, Human, Proposal},
};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
pub struct Provider {
    pub calls: AtomicUsize,
    pub mode: AtomicUsize,
    pub entered: tokio::sync::Notify,
    pub released: tokio::sync::Notify,
}
#[async_trait::async_trait]
impl Connector for Provider {
    fn id(&self) -> &'static str {
        "github"
    }
    fn tools(&self) -> Vec<Tool> {
        connector_github::definitions::tools(&connector_github::Config {
            repositories: vec!["demo/repo".into()],
            token_file: Some("unused".into()),
            credential_source: "file".into(),
            timeout: std::time::Duration::from_secs(1),
        })
    }
    fn validate(&self, _: &str, p: &Value) -> Result<()> {
        if p["repository"] != "demo/repo" {
            return Err(Error::new("github_repository_denied", 403));
        }
        Ok(())
    }
    fn preview(&self, t: &str, p: &Value) -> Option<Value> {
        Some(
            json!({"repository":p["repository"],"title":p["title"],"body":p["body"],"operation":t}),
        )
    }
    async fn execute(&self, t: &str, p: &Value, _: &str) -> Result<Value> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mode = self.mode.load(Ordering::SeqCst);
        if mode == 1 {
            return Err(Error::new("github_access_denied", 403));
        }
        if mode == 2 {
            return Err(Error::uncertain("github_timeout", 504));
        }
        if mode == 3 {
            return Ok(json!({"unexpected":"provider-secret"}));
        }
        if mode == 4 {
            self.entered.notify_one();
            self.released.notified().await;
        }
        let issue = json!({"number":7,"title":p["title"].as_str().unwrap_or("Read title"),"url":"https://github.com/demo/repo/issues/7","state":"open"});
        Ok(if t == "issues.create" {
            json!({"issue":issue})
        } else {
            json!({"issues":[issue]})
        })
    }
    fn receipt(&self, r: &Value) -> Option<Value> {
        Some(if r.get("issue").is_some() {
            json!({"provider":"github","issue_number":7,"issue_url":"https://github.com/demo/repo/issues/7"})
        } else {
            json!({"provider":"github","issue_count":1})
        })
    }
}
pub struct Harness {
    pub app: Arc<App>,
    pub provider: Arc<Provider>,
    pub origin: String,
    pub client: reqwest::Client,
    pub actor: Human,
    pub cookie: String,
    pub agent: String,
    pub credential: String,
    pub token: String,
    pub server: tokio::task::JoinHandle<()>,
    pub temp: tempfile::TempDir,
}
impl Drop for Harness {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Harness {
    pub async fn new() -> Self {
        Self::configured(true).await
    }
    pub async fn configured(execution: bool) -> Self {
        Self::with_ttl(execution, 900).await
    }
    pub async fn with_ttl(execution: bool, ttl: u64) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let provider = Arc::new(Provider {
            calls: AtomicUsize::new(0),
            mode: AtomicUsize::new(0),
            entered: tokio::sync::Notify::new(),
            released: tokio::sync::Notify::new(),
        });
        let mut config = Config::local(temp.path().join("runtime"));
        config.execution_enabled = execution;
        config.approval_seconds = ttl;
        config.origin = origin.clone();
        let app = App::new(config, vec![provider.clone()]).await.unwrap();
        control_core::identity::create_admin(&app, "admin", "Foundation9!Secure", "admin")
            .await
            .unwrap();
        control_core::identity::create_admin(&app, "reviewer", "Foundation9!Review", "reviewer")
            .await
            .unwrap();
        let router = control_server::router(app.clone());
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .await
            .unwrap();
        });
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let login = client
            .post(format!("{origin}/api/v1/auth/login"))
            .header("x-control-request", "1")
            .json(&json!({"username":"admin","password":"Foundation9!Secure"}))
            .send()
            .await
            .unwrap();
        assert_eq!(login.status(), 200);
        let cookie = login.headers()["set-cookie"]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string();
        let actor: Human = login.json().await.unwrap();
        let agent = control_core::agents::create(
            &app,
            &actor,
            control_core::agents::Create {
                name: "Test agent".into(),
                description: "".into(),
                provider: "".into(),
                runtime: "".into(),
                scopes: vec!["issues:read".into(), "issues:write".into()],
            },
        )
        .await
        .unwrap();
        let agent = agent["id"].as_str().unwrap().to_string();
        let cred = control_core::agents::issue(
            &app,
            &actor,
            &agent,
            vec!["issues:read".into(), "issues:write".into()],
        )
        .await
        .unwrap();
        control_core::lifecycle::connector(&app, &actor, "github", true)
            .await
            .unwrap();
        Self {
            app,
            provider,
            origin,
            client,
            actor,
            cookie,
            agent,
            credential: cred["id"].as_str().unwrap().into(),
            token: cred["bearer_token"].as_str().unwrap().into(),
            server,
            temp,
        }
    }
    pub fn human(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}{path}", self.origin))
            .header("cookie", &self.cookie)
            .header("x-csrf-token", &self.actor.csrf)
            .header("x-control-request", "1")
    }
    pub fn agent_request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}{path}", self.origin))
            .bearer_auth(&self.token)
    }
    pub async fn policy(&self, tool: &str, mode: &str) {
        let r = self
            .human(reqwest::Method::PUT, "/api/v1/policies")
            .json(&json!({"agent_id":self.agent,"connector":"github","tool":tool,"mode":mode}))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 200);
    }
    pub fn proposal(&self, key: &str) -> Proposal {
        serde_json::from_value(json!({"connector":"github","tool":"issues.create","parameters":{"repository":"demo/repo","title":"Exact private title","body":"Exact private body"},"idempotency_key":key})).unwrap()
    }
    pub async fn execute(&self, p: &Proposal) -> (u16, Value) {
        let r = self
            .agent_request(reqwest::Method::POST, "/api/v1/gateway/execute")
            .json(p)
            .send()
            .await
            .unwrap();
        let code = r.status().as_u16();
        (code, r.json().await.unwrap())
    }
    pub async fn approved(&self, key: &str) -> Proposal {
        self.policy("issues.create", "ask").await;
        let mut p = self.proposal(key);
        let (code, value) = self.execute(&p).await;
        assert_eq!(code, 202);
        let aid = value["approval_id"].as_str().unwrap();
        let r = self
            .human(
                reqwest::Method::POST,
                &format!("/api/v1/approvals/{aid}/decision"),
            )
            .json(&json!({"decision":"approved"}))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 200);
        p.approval_id = Some(aid.into());
        p
    }
    pub fn calls(&self) -> usize {
        self.provider.calls.load(Ordering::SeqCst)
    }
}
