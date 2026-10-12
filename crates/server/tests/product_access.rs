mod common;
use common::Harness;
use reqwest::Method;
use serde_json::{Value, json};
async fn access(h: &Harness) -> Value {
    h.human(Method::GET, &format!("/api/v1/agents/{}/access", h.agent))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}
fn read(v: &Value) -> &Value {
    v["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["tool"] == "issues.list")
        .unwrap()
}
#[tokio::test]
async fn diagnostics_follow_current_policy_credential_and_scope_controls() {
    let h = Harness::new().await;
    assert_eq!(read(&access(&h).await)["reason"], "default_deny");
    h.policy("issues.list", "allow").await;
    assert_eq!(read(&access(&h).await)["effective_policy"], "allow");
    let response = h
        .human(Method::PATCH, &format!("/api/v1/agents/{}/scopes", h.agent))
        .json(&json!({"scopes":[]}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 204);
    assert_eq!(read(&access(&h).await)["reason"], "scope_missing");
    let mut p = h.proposal("access-shrink-001");
    p.tool = "issues.list".into();
    p.parameters = json!({"repository":"demo/repo"})
        .as_object()
        .unwrap()
        .clone();
    assert_eq!(h.execute(&p).await.0, 403);
    assert_eq!(h.calls(), 0);
    control_core::agents::set_scopes(&h.app, &h.actor, &h.agent, vec!["issues:read".into()])
        .await
        .unwrap();
    assert_eq!(read(&access(&h).await)["effective_policy"], "allow");
    control_core::agents::revoke(&h.app, &h.actor, &h.agent, &h.credential)
        .await
        .unwrap();
    assert_eq!(read(&access(&h).await)["reason"], "no_active_credential");
}
#[tokio::test]
async fn diagnostics_do_not_misrepresent_mandatory_confirmation_or_global_stop() {
    let h = Harness::new().await;
    h.policy("issues.create", "allow").await;
    let v = access(&h).await;
    let create = v["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["tool"] == "issues.create")
        .unwrap();
    assert_eq!(create["explicit_policy"], "allow");
    assert_eq!(create["effective_policy"], "ask");
    control_core::lifecycle::stop(&h.app, &h.actor, true)
        .await
        .unwrap();
    assert_eq!(read(&access(&h).await)["reason"], "emergency_stop");
}
#[tokio::test]
async fn catalog_and_diagnostics_do_not_authenticate_agents_as_humans() {
    let h = Harness::new().await;
    for path in [
        "/api/v1/catalog".to_string(),
        format!("/api/v1/agents/{}/access", h.agent),
    ] {
        assert_eq!(
            h.agent_request(Method::GET, &path)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
    }
    let catalog: Value = h
        .human(Method::GET, "/api/v1/catalog")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        catalog["connectors"][0]["tools"].as_array().unwrap().len(),
        2
    );
    assert_eq!(catalog["mcp_url"], format!("{}/mcp/", h.origin));
    assert!(!catalog.to_string().contains(&h.token));
}
#[tokio::test]
async fn scope_changes_require_admin_csrf_and_valid_scopes() {
    let h = Harness::new().await;
    let path = format!("/api/v1/agents/{}/scopes", h.agent);
    assert_eq!(
        h.agent_request(Method::PATCH, &path)
            .json(&json!({"scopes":[]}))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        h.client
            .patch(format!("{}{path}", h.origin))
            .header("cookie", &h.cookie)
            .json(&json!({"scopes":[]}))
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        h.human(Method::PATCH, &path)
            .json(&json!({"scopes":["*"]}))
            .send()
            .await
            .unwrap()
            .status(),
        422
    );
    let login = h
        .client
        .post(format!("{}/api/v1/auth/login", h.origin))
        .header("x-control-request", "1")
        .json(&json!({"username":"reviewer","password":"Foundation9!Review"}))
        .send()
        .await
        .unwrap();
    let cookie = login.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let actor: Value = login.json().await.unwrap();
    assert_eq!(
        h.client
            .patch(format!("{}{path}", h.origin))
            .header("cookie", cookie)
            .header("x-csrf-token", actor["csrf"].as_str().unwrap())
            .json(&json!({"scopes":[]}))
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
}
