mod common;
use common::Harness;
use reqwest::Method;
use serde_json::{Value, json};
#[tokio::test]
async fn session_workflow() {
    let h = Harness::new().await;
    let me = h
        .human(Method::GET, "/api/v1/auth/me")
        .send()
        .await
        .unwrap();
    assert_eq!(me.status(), 200);
    let me: Value = me.json().await.unwrap();
    assert_eq!(me["role"], "admin");
    assert_eq!(
        h.human(Method::POST, "/api/v1/auth/logout")
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    assert_eq!(
        h.human(Method::GET, "/api/v1/auth/me")
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
}
#[tokio::test]
async fn agent_cannot_approve() {
    let h = Harness::new().await;
    h.policy("issues.create", "ask").await;
    let (_, v) = h.execute(&h.proposal("agent-approve-001")).await;
    let path = format!(
        "/api/v1/approvals/{}/decision",
        v["approval_id"].as_str().unwrap()
    );
    assert_eq!(
        h.agent_request(Method::POST, &path)
            .json(&json!({"decision":"approved"}))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(h.calls(), 0);
}
#[tokio::test]
async fn human_cookie_cannot_execute_as_agent() {
    let h = Harness::new().await;
    assert_eq!(
        h.human(Method::POST, "/api/v1/gateway/execute")
            .json(&h.proposal("cookie-execute-001"))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
}
#[tokio::test]
async fn csrf_required() {
    let h = Harness::new().await;
    let r = h
        .client
        .post(format!("{}/api/v1/agents", h.origin))
        .header("cookie", &h.cookie)
        .json(&json!({"name":"no csrf"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 403);
}
#[tokio::test]
async fn stale_session_rejected() {
    let h = Harness::new().await;
    sqlx::query("UPDATE sessions SET expires_at=0")
        .execute(&h.app.db.pool)
        .await
        .unwrap();
    assert_eq!(
        h.human(Method::GET, "/api/v1/auth/me")
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
}
#[tokio::test]
async fn disabled_human_session_rejected() {
    let h = Harness::new().await;
    sqlx::query("UPDATE humans SET enabled=0 WHERE id=?")
        .bind(&h.actor.id)
        .execute(&h.app.db.pool)
        .await
        .unwrap();
    assert_eq!(
        h.human(Method::GET, "/api/v1/auth/me")
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
}
#[tokio::test]
async fn reviewer_cannot_manage_agents() {
    let h = Harness::new().await;
    let r = h
        .client
        .post(format!("{}/api/v1/auth/login", h.origin))
        .header("x-control-request", "1")
        .json(&json!({"username":"reviewer","password":"Foundation9!Review"}))
        .send()
        .await
        .unwrap();
    let cookie = r.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    assert_eq!(
        h.client
            .get(format!("{}/api/v1/agents", h.origin))
            .header("cookie", cookie)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
}
#[tokio::test]
async fn reviewer_can_decide() {
    let h = Harness::new().await;
    h.policy("issues.create", "ask").await;
    let (_, v) = h.execute(&h.proposal("reviewer-approve-001")).await;
    let r = h
        .client
        .post(format!("{}/api/v1/auth/login", h.origin))
        .header("x-control-request", "1")
        .json(&json!({"username":"reviewer","password":"Foundation9!Review"}))
        .send()
        .await
        .unwrap();
    let cookie = r.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let actor: Value = r.json().await.unwrap();
    let r = h
        .client
        .post(format!(
            "{}/api/v1/approvals/{}/decision",
            h.origin,
            v["approval_id"].as_str().unwrap()
        ))
        .header("cookie", cookie)
        .header("x-csrf-token", actor["csrf"].as_str().unwrap())
        .json(&json!({"decision":"approved"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
}
#[tokio::test]
async fn login_rate_limit() {
    let h = Harness::new().await;
    for i in 0..6 {
        let r = h
            .client
            .post(format!("{}/api/v1/auth/login", h.origin))
            .header("x-control-request", "1")
            .json(&json!({"username":"missing","password":"invalid"}))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status().as_u16(), if i < 5 { 401 } else { 429 });
    }
}
#[tokio::test]
async fn malformed_login_does_not_echo_password() {
    let h = Harness::new().await;
    let r = h
        .client
        .post(format!("{}/api/v1/auth/login", h.origin))
        .header("x-control-request", "1")
        .json(&json!({"username":"admin","password":"SECRET_TEST_ONLY","extra":true}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 422);
    assert!(!r.text().await.unwrap().contains("SECRET_TEST_ONLY"));
}
#[tokio::test]
async fn origin_rejected() {
    let h = Harness::new().await;
    assert_eq!(
        h.human(Method::POST, "/api/v1/agents")
            .header("origin", "https://evil.example")
            .json(&json!({"name":"blocked"}))
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
}
#[tokio::test]
async fn host_rejected() {
    let h = Harness::new().await;
    assert_eq!(
        h.client
            .get(format!("{}/health", h.origin))
            .header("host", "evil.example")
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
}
#[tokio::test]
async fn frontend_and_admin_compatibility() {
    let h = Harness::new().await;
    for path in [
        "/",
        "/ui/src/app.js",
        "/ui/src/styles/app.css",
        "/ui/favicon.svg",
        "/health",
    ] {
        let r = h
            .client
            .get(format!("{}{path}", h.origin))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 200);
        assert_eq!(r.headers()["x-content-type-options"], "nosniff");
    }
    for path in [
        "/api/v1/agents",
        "/api/v1/policies",
        "/api/v1/approvals",
        "/api/v1/audit",
        "/api/v1/integrations",
        "/api/v1/settings",
    ] {
        assert_eq!(
            h.human(Method::GET, path).send().await.unwrap().status(),
            200
        );
    }
    let r = h
        .human(Method::GET, &format!("/api/v1/agents/{}", h.agent))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    assert!(r["credentials"][0].get("token_hash").is_none());
}
#[tokio::test]
async fn request_limit() {
    let h = Harness::new().await;
    let r = h
        .human(Method::POST, "/api/v1/agents")
        .header("content-type", "application/json")
        .body("x".repeat(32769))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 413);
}
#[tokio::test]
async fn invalid_scopes_rejected() {
    let h = Harness::new().await;
    let r = h
        .human(Method::POST, "/api/v1/agents")
        .json(&json!({"name":"bad","scopes":["*"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 422);
}
#[tokio::test]
async fn password_work_is_bounded() {
    let h = Harness::new().await;
    let _permits = h
        .app
        .password_slots
        .clone()
        .acquire_many_owned(4)
        .await
        .unwrap();
    let r = h
        .client
        .post(format!("{}/api/v1/auth/login", h.origin))
        .header("x-control-request", "1")
        .json(&json!({"username":"admin","password":"Foundation9!Secure"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 429);
    assert_eq!(
        r.json::<Value>().await.unwrap()["error"],
        "authentication_busy"
    );
}
