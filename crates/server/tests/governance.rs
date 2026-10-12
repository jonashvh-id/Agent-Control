mod common;
use common::Harness;
use control_core::models::Policy;
use reqwest::Method;
use serde_json::json;
#[tokio::test]
async fn default_deny() {
    let h = Harness::new().await;
    assert_eq!(h.execute(&h.proposal("default-deny-001")).await.0, 403);
    assert_eq!(h.calls(), 0);
}
#[tokio::test]
async fn creation_allow_still_asks() {
    let h = Harness::new().await;
    h.policy("issues.create", "allow").await;
    assert_eq!(h.execute(&h.proposal("allow-ask-001")).await.0, 202);
    assert_eq!(h.calls(), 0);
}
#[tokio::test]
async fn explicit_allow_read() {
    let h = Harness::new().await;
    h.policy("issues.list", "allow").await;
    let mut p = h.proposal("allow-read-001");
    p.tool = "issues.list".into();
    p.parameters = json!({"repository":"demo/repo"})
        .as_object()
        .unwrap()
        .clone();
    assert_eq!(h.execute(&p).await.0, 200);
    assert_eq!(h.calls(), 1);
}
#[tokio::test]
async fn exact_approval_executes_and_replay_never_writes() {
    let h = Harness::new().await;
    let p = h.approved("exact-key-001").await;
    assert_eq!(h.execute(&p).await.0, 200);
    let (status, body) = h.execute(&p).await;
    assert_eq!(status, 200);
    assert_eq!(body["status"], "already_executed");
    assert_eq!(h.calls(), 1);
}
#[tokio::test]
async fn changed_title_rejected() {
    let h = Harness::new().await;
    let mut p = h.approved("changed-title-001").await;
    p.parameters["title"] = json!("Changed");
    assert_eq!(h.execute(&p).await.0, 403);
    assert_eq!(h.calls(), 0);
}
#[tokio::test]
async fn changed_body_rejected() {
    let h = Harness::new().await;
    let mut p = h.approved("changed-body-001").await;
    p.parameters["body"] = json!("Changed");
    assert_eq!(h.execute(&p).await.0, 403);
    assert_eq!(h.calls(), 0);
}
#[tokio::test]
async fn changed_key_rejected() {
    let h = Harness::new().await;
    let mut p = h.approved("changed-key-001").await;
    p.idempotency_key = "changed-key-002".into();
    assert_eq!(h.execute(&p).await.0, 403);
    assert_eq!(h.calls(), 0);
}
#[tokio::test]
async fn different_credential_cannot_consume() {
    let mut h = Harness::new().await;
    let p = h.approved("credential-bind-001").await;
    let c = control_core::agents::issue(&h.app, &h.actor, &h.agent, vec!["issues:write".into()])
        .await
        .unwrap();
    h.token = c["bearer_token"].as_str().unwrap().into();
    assert_eq!(h.execute(&p).await.0, 403);
    assert_eq!(h.calls(), 0);
}
#[tokio::test]
async fn approval_expiry_blocks() {
    let h = Harness::with_ttl(true, 1).await;
    let p = h.approved("expired-key-001").await;
    let mut db = h.app.db.write().await.unwrap();
    assert!(
        sqlx::query("UPDATE approvals SET expires_at=0")
            .execute(&mut *db)
            .await
            .is_err()
    );
    db.rollback().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    assert_eq!(h.execute(&p).await.0, 409);
    assert_eq!(h.calls(), 0);
}
#[tokio::test]
async fn consumed_approval_new_execution_rejected() {
    let h = Harness::new().await;
    let mut p = h.approved("consumed-key-001").await;
    assert_eq!(h.execute(&p).await.0, 200);
    p.idempotency_key = "consumed-key-002".into();
    assert_eq!(h.execute(&p).await.0, 403);
    assert_eq!(h.calls(), 1);
}
async fn tightened(kind: &str) {
    let h = Harness::new().await;
    let p = h.approved("tightened-key-001").await;
    match kind {
        "credential" => control_core::agents::revoke(&h.app, &h.actor, &h.agent, &h.credential)
            .await
            .unwrap(),
        "agent" => control_core::lifecycle::agent(&h.app, &h.actor, &h.agent, "disabled")
            .await
            .unwrap(),
        "connector" => control_core::lifecycle::connector(&h.app, &h.actor, "github", false)
            .await
            .unwrap(),
        "global" => control_core::lifecycle::stop(&h.app, &h.actor, true)
            .await
            .unwrap(),
        "approver" => {
            sqlx::query("UPDATE humans SET enabled=0 WHERE id=?")
                .bind(&h.actor.id)
                .execute(&h.app.db.pool)
                .await
                .unwrap();
        }
        _ => h.policy("issues.create", "deny").await,
    };
    assert!(matches!(h.execute(&p).await.0, 401 | 403));
    assert_eq!(h.calls(), 0);
}
macro_rules! tightening {
    ($name:ident,$kind:literal) => {
        #[tokio::test]
        async fn $name() {
            tightened($kind).await;
        }
    };
}
tightening!(revoked_credential, "credential");
tightening!(disabled_agent, "agent");
tightening!(disabled_connector, "connector");
tightening!(global_stop, "global");
tightening!(disabled_approver, "approver");
tightening!(current_policy_deny, "policy");
#[tokio::test]
async fn scopes_cannot_be_overridden_by_allow() {
    let h = Harness::new().await;
    h.policy("issues.list", "allow").await;
    sqlx::query("UPDATE credentials SET scopes='[]'")
        .execute(&h.app.db.pool)
        .await
        .unwrap();
    let r = h
        .agent_request(Method::GET, "/api/v1/gateway/tools")
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(r["tools"], json!([]));
}
#[tokio::test]
async fn credential_escalation_rejected() {
    let h = Harness::new().await;
    let r = h
        .human(
            Method::POST,
            &format!("/api/v1/agents/{}/credentials", h.agent),
        )
        .json(&json!({"scopes":["repos:delete"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 403);
}
#[tokio::test]
async fn unregistered_delete_rejected() {
    let h = Harness::new().await;
    let mut p = h.proposal("delete-key-001");
    p.tool = "repos.delete".into();
    assert_eq!(h.execute(&p).await.0, 404);
    assert_eq!(h.calls(), 0);
}
#[tokio::test]
async fn disabled_execution_refuses() {
    let h = Harness::configured(false).await;
    h.policy("issues.list", "allow").await;
    let mut p = h.proposal("disabled-read-001");
    p.tool = "issues.list".into();
    assert_eq!(h.execute(&p).await.0, 503);
    assert_eq!(h.calls(), 0);
}
#[tokio::test]
async fn preview_encrypted_and_audit_minimized() {
    let h = Harness::new().await;
    let p = h.approved("encrypted-key-001").await;
    assert_eq!(h.execute(&p).await.0, 200);
    let raw: Vec<String> = sqlx::query_scalar("SELECT preview_ciphertext FROM approvals")
        .fetch_all(&h.app.db.pool)
        .await
        .unwrap();
    assert!(!raw.join("").contains("Exact private"));
    let events = control_core::audit::list(&h.app.db.pool)
        .await
        .unwrap()
        .to_string();
    for value in [&h.token, "Exact private title", "Exact private body"] {
        assert!(!events.contains(value));
    }
}
#[test]
fn policy_scope_intersection_unit() {
    let c = common::Provider {
        calls: 0.into(),
        mode: 0.into(),
        entered: Default::default(),
        released: Default::default(),
    };
    use control_core::connector::Connector;
    let tool = &c.tools()[1];
    assert_eq!(
        control_core::policy::effective(tool, &[], Policy::Allow),
        Policy::Deny
    );
    assert_eq!(
        control_core::policy::effective(tool, &["issues:write".into()], Policy::Allow),
        Policy::Ask
    );
}
