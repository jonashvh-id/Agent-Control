mod common;
use common::Harness;
use control_core::{
    models::{Policy, Proposal},
    policy::Set,
};
use reqwest::Method;
use serde_json::{Value, json};
async fn drive_policy(h: &Harness, tool: &str, mode: Policy) {
    control_core::policy::set(
        &h.app,
        &h.actor,
        Set {
            agent_id: h.agent.clone(),
            connector: "google-drive".into(),
            tool: tool.into(),
            mode,
        },
    )
    .await
    .unwrap();
}
#[tokio::test]
async fn discovery_intersects_each_application_with_current_permissions() {
    let h = Harness::multi().await;
    h.policy("issues.list", "allow").await;
    let mut tools: Value = h
        .agent_request(Method::GET, "/api/v1/gateway/tools")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(tools["tools"].as_array().unwrap().len(), 1);
    drive_policy(&h, "files.search", Policy::Allow).await;
    tools = h
        .agent_request(Method::GET, "/api/v1/gateway/tools")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(tools["tools"].as_array().unwrap().len(), 2);
    control_core::lifecycle::connector(&h.app, &h.actor, "google-drive", false)
        .await
        .unwrap();
    tools = h
        .agent_request(Method::GET, "/api/v1/gateway/tools")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(tools["tools"].as_array().unwrap().len(), 1);
}
#[tokio::test]
async fn approval_cannot_cross_applications_and_revocation_blocks_both() {
    let h = Harness::multi().await;
    h.policy("issues.create", "ask").await;
    drive_policy(&h, "files.read", Policy::Ask).await;
    let mut p:Proposal=serde_json::from_value(json!({"connector":"google-drive","tool":"files.read","parameters":{"file_id":common::drive::FILE},"idempotency_key":"two-app-key-001"})).unwrap();
    let (status, pending) = h.execute(&p).await;
    assert_eq!(status, 202);
    let aid = pending["approval_id"].as_str().unwrap();
    control_core::approvals::decide(&h.app, &h.actor, aid, "approved")
        .await
        .unwrap();
    let mut other = h.proposal("two-app-key-001");
    other.approval_id = Some(aid.into());
    assert_eq!(h.execute(&other).await.0, 403);
    assert_eq!(h.calls(), 0);
    p.approval_id = Some(aid.into());
    assert_eq!(h.execute(&p).await.0, 200);
    assert_eq!(
        h.drive
            .as_ref()
            .unwrap()
            .calls
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );
    control_core::agents::revoke(&h.app, &h.actor, &h.agent, &h.credential)
        .await
        .unwrap();
    assert_eq!(h.execute(&p).await.0, 401);
    assert_eq!(h.execute(&other).await.0, 401);
}
