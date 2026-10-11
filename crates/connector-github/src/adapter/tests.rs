use super::*;
use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, routing::any};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
struct Fixture {
    adapter: GitHub,
    calls: Arc<AtomicUsize>,
    server: tokio::task::JoinHandle<()>,
    temp: tempfile::TempDir,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
async fn fixture(status: u16, body: String, delay: u64) -> Fixture {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let route = Router::new()
        .fallback(any(
            move |State(count): State<Arc<AtomicUsize>>, request: axum::extract::Request| {
                let body = body.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(request.uri().path(), "/repos/demo/repo/issues");
                    assert_eq!(
                        request.headers()["authorization"],
                        "Bearer fixture-private-token"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    (StatusCode::from_u16(status).unwrap(), body).into_response()
                }
            },
        ))
        .with_state(count);
    let server = tokio::spawn(async move {
        axum::serve(listener, route).await.unwrap();
    });
    let temp = tempfile::tempdir().unwrap();
    let token_file = temp.path().join("github.token");
    control_core::security::files::create_private(&token_file, b"fixture-private-token").unwrap();
    let config = Config {
        repositories: vec!["demo/repo".into()],
        token_file: Some(token_file),
        credential_source: "file".into(),
        timeout: std::time::Duration::from_millis(100),
    };
    let client = Client::builder()
        .timeout(config.timeout)
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .no_proxy()
        .build()
        .unwrap();
    Fixture {
        adapter: GitHub {
            config,
            client,
            origin,
        },
        calls,
        server,
        temp,
    }
}
fn issue() -> Value {
    json!({"number":7,"title":"Untrusted provider title","html_url":"https://github.com/demo/repo/issues/7","state":"open","body":"NEVER_COPY_PAYLOAD","token":"NEVER_COPY_SECRET"})
}
fn input() -> Value {
    json!({"repository":"demo/repo","title":"Exact title","body":"Exact body"})
}
#[tokio::test]
async fn issue_creation_minimizes_response() {
    let f = fixture(201, issue().to_string(), 0).await;
    let result = f
        .adapter
        .execute("issues.create", &input(), "key")
        .await
        .unwrap();
    assert_eq!(result["issue"]["number"], 7);
    assert!(!result.to_string().contains("NEVER_COPY"));
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn issue_listing_excludes_pull_requests() {
    let mut pr = issue();
    pr["pull_request"] = json!({});
    let f = fixture(200, json!([issue(), pr]).to_string(), 0).await;
    let result = f
        .adapter
        .execute("issues.list", &json!({"repository":"demo/repo"}), "key")
        .await
        .unwrap();
    assert_eq!(result["issues"].as_array().unwrap().len(), 1);
}
async fn denial(status: u16, code: &str) {
    let f = fixture(status, "token=NEVER_EXPOSE_SECRET".into(), 0).await;
    let error = f
        .adapter
        .execute("issues.create", &input(), "key")
        .await
        .unwrap_err();
    assert_eq!(error.code, code);
    assert!(!error.to_string().contains("NEVER_EXPOSE"));
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
}
macro_rules! status_test {
    ($name:ident,$status:literal,$code:literal) => {
        #[tokio::test]
        async fn $name() {
            denial($status, $code).await;
        }
    };
}
status_test!(native_401, 401, "github_access_denied");
status_test!(native_403, 403, "github_access_denied");
status_test!(native_404, 404, "github_access_denied");
status_test!(rate_limit, 429, "github_rate_limited");
status_test!(provider_failure, 500, "github_request_failed");
status_test!(redirect_refused, 302, "github_request_failed");
#[tokio::test]
async fn real_provider_timeout_is_uncertain() {
    let f = fixture(201, issue().to_string(), 300).await;
    let e = f
        .adapter
        .execute("issues.create", &input(), "key")
        .await
        .unwrap_err();
    assert_eq!(e.code, "github_timeout");
    assert!(e.uncertain);
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn malformed_provider_json_is_uncertain() {
    let f = fixture(201, "not json".into(), 0).await;
    assert!(
        f.adapter
            .execute("issues.create", &input(), "key")
            .await
            .unwrap_err()
            .uncertain
    );
}
#[tokio::test]
async fn bounded_provider_response() {
    let f = fixture(201, "x".repeat(262145), 0).await;
    assert_eq!(
        f.adapter
            .execute("issues.create", &input(), "key")
            .await
            .unwrap_err()
            .code,
        "github_result_too_large"
    );
}
#[tokio::test]
async fn url_query_fragment_and_wrong_origin_refused() {
    let f = fixture(201, issue().to_string(), 0).await;
    for url in [
        "https://github.com/demo/repo/issues/7?secret=x",
        "https://github.com/demo/repo/issues/7#secret",
        "https://evil.example/demo/repo/issues/7",
        "https://github.com/other/repo/issues/7",
        "https://user:pass@github.com/demo/repo/issues/7",
    ] {
        let mut value = issue();
        value["html_url"] = json!(url);
        assert!(f.adapter.issue(&value, "demo/repo").is_err());
    }
}
#[tokio::test]
async fn repository_escape_refused_before_io() {
    let f = fixture(201, issue().to_string(), 0).await;
    for repo in [
        "other/repo",
        "demo/../../evil",
        "https://evil.example",
        "jonashvh-id/zetako-node",
    ] {
        let mut p = input();
        p["repository"] = json!(repo);
        assert!(f.adapter.execute("issues.create", &p, "key").await.is_err());
    }
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn secret_file_rotation() {
    let f = fixture(201, issue().to_string(), 0).await;
    let path = f.adapter.config.token_file.as_ref().unwrap();
    std::fs::write(path, "rotated-private-token").unwrap();
    assert_eq!(
        *credentials::token(&f.adapter.config).await.unwrap(),
        "rotated-private-token"
    );
}
#[cfg(unix)]
#[tokio::test]
async fn secret_file_permissions_required() {
    use std::os::unix::fs::PermissionsExt;
    let f = fixture(201, issue().to_string(), 0).await;
    std::fs::set_permissions(
        f.adapter.config.token_file.as_ref().unwrap(),
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert!(credentials::token(&f.adapter.config).await.is_err());
    assert!(
        f.adapter
            .execute("issues.create", &input(), "key")
            .await
            .is_err()
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
}
#[cfg(unix)]
#[tokio::test]
async fn symlink_secret_refused() {
    let mut f = fixture(201, issue().to_string(), 0).await;
    let link = f.temp.path().join("link.token");
    std::os::unix::fs::symlink(f.adapter.config.token_file.as_ref().unwrap(), &link).unwrap();
    f.adapter.config.token_file = Some(link);
    assert!(credentials::token(&f.adapter.config).await.is_err());
}
#[test]
fn protected_project_and_demo_config_refused() {
    let config = |repo: &str, source: &str| Config {
        repositories: vec![repo.into()],
        token_file: Some("unused".into()),
        credential_source: source.into(),
        timeout: std::time::Duration::from_secs(1),
    };
    assert!(
        config("jonashvh-id/zetako-node", "file")
            .validate(true)
            .is_err()
    );
    assert!(config("demo/repo", "gh-demo").validate(true).is_err());
    assert!(
        config("jonashvh-id/Agent-Control", "gh-demo")
            .validate(false)
            .is_err()
    );
    assert!(
        config("jonashvh-id/Agent-Control", "gh-demo")
            .validate(true)
            .is_ok()
    );
}
#[tokio::test]
async fn actual_adapter_respects_domain_approval_and_native_denial() {
    let mut f = fixture(403, "native denial".into(), 0).await;
    let substitute = GitHub::new(f.adapter.config.clone(), true).unwrap();
    let adapter = std::mem::replace(&mut f.adapter, substitute);
    let mut config = control_core::models::Config::local(f.temp.path().join("runtime"));
    config.execution_enabled = true;
    let app = control_core::App::new(config, vec![Arc::new(adapter)])
        .await
        .unwrap();
    let human = control_core::identity::create_admin(&app, "admin", "Foundation9!Secure", "admin")
        .await
        .unwrap();
    let actor = control_core::models::Human {
        id: human,
        username: "admin".into(),
        role: "admin".into(),
        csrf: "".into(),
    };
    let agent = control_core::agents::create(
        &app,
        &actor,
        control_core::agents::Create {
            name: "Pipeline".into(),
            description: "".into(),
            provider: "".into(),
            runtime: "".into(),
            scopes: vec!["issues:write".into()],
        },
    )
    .await
    .unwrap();
    let aid = agent["id"].as_str().unwrap();
    let credential = control_core::agents::issue(&app, &actor, aid, vec!["issues:write".into()])
        .await
        .unwrap();
    let token = credential["bearer_token"].as_str().unwrap();
    control_core::lifecycle::connector(&app, &actor, "github", true)
        .await
        .unwrap();
    control_core::policy::set(
        &app,
        &actor,
        control_core::policy::Set {
            agent_id: aid.into(),
            connector: "github".into(),
            tool: "issues.create".into(),
            mode: control_core::models::Policy::Allow,
        },
    )
    .await
    .unwrap();
    let mut proposal:control_core::models::Proposal=serde_json::from_value(json!({"connector":"github","tool":"issues.create","parameters":input(),"idempotency_key":"real-adapter-key"})).unwrap();
    let pending = control_core::gateway::execute(&app, token, proposal.clone())
        .await
        .unwrap();
    assert_eq!(pending.status, 202);
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    let approval = pending.body["approval_id"].as_str().unwrap();
    control_core::approvals::decide(&app, &actor, approval, "approved")
        .await
        .unwrap();
    proposal.approval_id = Some(approval.into());
    let result = control_core::gateway::execute(&app, token, proposal.clone())
        .await
        .unwrap();
    assert_eq!(result.status, 403);
    assert_eq!(result.body["error"], "github_access_denied");
    assert_eq!(
        control_core::gateway::execute(&app, token, proposal)
            .await
            .unwrap()
            .status,
        409
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn numeric_read_arguments_not_silently_defaulted() {
    let f = fixture(200, json!([]).to_string(), 0).await;
    let p = json!({"repository":"demo/repo","limit":1.0});
    assert_eq!(
        f.adapter
            .execute("issues.list", &p, "key")
            .await
            .unwrap_err()
            .code,
        "invalid_tool_parameters"
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
}
