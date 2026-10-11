mod common;
use common::Harness;
use serde_json::json;
use std::sync::atomic::Ordering;
#[tokio::test]
async fn concurrent_duplicates_dispatch_once() {
    let h = Harness::new().await;
    let p = h.approved("concurrent-key-001").await;
    let (a, b, c) = tokio::join!(h.execute(&p), h.execute(&p), h.execute(&p));
    for status in [a.0, b.0, c.0] {
        assert!(matches!(status, 200 | 409));
    }
    assert_eq!(h.calls(), 1);
}
#[tokio::test]
async fn idempotency_conflict() {
    let h = Harness::new().await;
    let mut p = h.approved("conflict-key-001").await;
    assert_eq!(h.execute(&p).await.0, 200);
    p.parameters["title"] = json!("Different");
    assert_eq!(h.execute(&p).await.0, 409);
    assert_eq!(h.calls(), 1);
}
#[tokio::test]
async fn provider_native_denial_never_retries() {
    let h = Harness::new().await;
    h.provider.mode.store(1, Ordering::SeqCst);
    let p = h.approved("native-denial-001").await;
    let (code, body) = h.execute(&p).await;
    assert_eq!(code, 403);
    assert_eq!(body["execution_state"], "failed");
    assert_eq!(h.execute(&p).await.0, 409);
    assert_eq!(h.calls(), 1);
}
#[tokio::test]
async fn timeout_outcome_is_explicitly_uncertain() {
    let h = Harness::new().await;
    h.provider.mode.store(2, Ordering::SeqCst);
    let p = h.approved("timeout-key-001").await;
    let (code, body) = h.execute(&p).await;
    assert_eq!(code, 504);
    assert_eq!(body["execution_state"], "uncertain");
    assert_eq!(h.execute(&p).await.0, 409);
    assert_eq!(h.calls(), 1);
}
#[tokio::test]
async fn untrusted_invalid_provider_response_is_uncertain() {
    let h = Harness::new().await;
    h.provider.mode.store(3, Ordering::SeqCst);
    let p = h.approved("invalid-output-001").await;
    let (code, body) = h.execute(&p).await;
    assert_eq!(code, 502);
    assert_eq!(body["execution_state"], "uncertain");
    assert!(!body.to_string().contains("provider-secret"));
    assert_eq!(h.execute(&p).await.0, 409);
    assert_eq!(h.calls(), 1);
}
#[tokio::test]
async fn emergency_stop_does_not_wait_for_network() {
    let h = Harness::new().await;
    h.provider.mode.store(4, Ordering::SeqCst);
    let p = h.approved("slow-key-001").await;
    let execute = h.execute(&p);
    let stop = async {
        h.provider.entered.notified().await;
        tokio::time::timeout(
            std::time::Duration::from_millis(500),
            control_core::lifecycle::stop(&h.app, &h.actor, true),
        )
        .await
        .unwrap()
        .unwrap();
        h.provider.released.notify_one();
    };
    let (result, _) = tokio::join!(execute, stop);
    assert_eq!(result.0, 200);
    assert_eq!(h.execute(&p).await.0, 403);
    assert_eq!(h.calls(), 1);
}
#[tokio::test]
async fn policy_rechecked_after_preparation() {
    let h = Harness::new().await;
    let p = h.approved("recheck-key-001").await;
    let prepared = control_core::gateway::admission::prepare(&h.app, &h.token, &p, "correlation")
        .await
        .unwrap();
    h.policy("issues.create", "deny").await;
    let control_core::gateway::admission::Admission::Prepared(prepared) = prepared else {
        panic!("not prepared")
    };
    let outcome =
        control_core::gateway::dispatch::execute(&h.app, &h.token, &p, "correlation", *prepared)
            .await
            .unwrap();
    assert_eq!(outcome.status, 403);
    assert_eq!(h.calls(), 0);
}
#[tokio::test]
async fn approver_rechecked_after_consumption() {
    let h = Harness::new().await;
    let p = h.approved("approver-recheck-001").await;
    let control_core::gateway::admission::Admission::Prepared(prepared) =
        control_core::gateway::admission::prepare(&h.app, &h.token, &p, "correlation")
            .await
            .unwrap()
    else {
        panic!()
    };
    sqlx::query("UPDATE humans SET enabled=0 WHERE id=?")
        .bind(&h.actor.id)
        .execute(&h.app.db.pool)
        .await
        .unwrap();
    let outcome =
        control_core::gateway::dispatch::execute(&h.app, &h.token, &p, "correlation", *prepared)
            .await
            .unwrap();
    assert_eq!(outcome.status, 403);
    assert_eq!(h.calls(), 0);
}
#[tokio::test]
async fn credential_rechecked_after_preparation() {
    let h = Harness::new().await;
    let p = h.approved("revoke-recheck-001").await;
    let control_core::gateway::admission::Admission::Prepared(prepared) =
        control_core::gateway::admission::prepare(&h.app, &h.token, &p, "correlation")
            .await
            .unwrap()
    else {
        panic!()
    };
    control_core::agents::revoke(&h.app, &h.actor, &h.agent, &h.credential)
        .await
        .unwrap();
    let outcome =
        control_core::gateway::dispatch::execute(&h.app, &h.token, &p, "correlation", *prepared)
            .await
            .unwrap();
    assert_eq!(outcome.status, 401);
    assert_eq!(h.calls(), 0);
}
#[tokio::test]
async fn approvals_binding_is_immutable() {
    let h = Harness::new().await;
    let _ = h.approved("immutable-key-001").await;
    for column in [
        "input_hash",
        "preview_ciphertext",
        "idempotency_key",
        "agent_id",
    ] {
        let q = format!("UPDATE approvals SET {column}='altered'");
        assert!(sqlx::query(&q).execute(&h.app.db.pool).await.is_err());
    }
}
#[tokio::test]
async fn audit_append_only() {
    let h = Harness::new().await;
    assert!(
        sqlx::query("UPDATE audit_events SET decision='allow'")
            .execute(&h.app.db.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM audit_events")
            .execute(&h.app.db.pool)
            .await
            .is_err()
    );
}
#[tokio::test]
async fn prepared_request_cannot_change_before_dispatch() {
    let h = Harness::new().await;
    let mut p = h.approved("dispatch-bind-001").await;
    let control_core::gateway::admission::Admission::Prepared(prepared) =
        control_core::gateway::admission::prepare(&h.app, &h.token, &p, "correlation")
            .await
            .unwrap()
    else {
        panic!()
    };
    p.parameters["title"] = json!("changed after consumption");
    let outcome =
        control_core::gateway::dispatch::execute(&h.app, &h.token, &p, "correlation", *prepared)
            .await
            .unwrap();
    assert_eq!(outcome.status, 403);
    assert_eq!(h.calls(), 0);
}
#[tokio::test]
async fn unregistered_agent_tool_names_not_logged() {
    let h = Harness::new().await;
    let mut p = h.proposal("unknown-tool-001");
    p.connector = "untrusted_private_value".into();
    p.tool = "secret.tool.value".into();
    assert_eq!(h.execute(&p).await.0, 404);
    let audit = control_core::audit::list(&h.app.db.pool)
        .await
        .unwrap()
        .to_string();
    assert!(!audit.contains("untrusted_private_value"));
    assert!(!audit.contains("secret.tool.value"));
}
#[tokio::test]
async fn provider_work_bounded_without_blocking_controls() {
    let h = Harness::new().await;
    let p = h.approved("capacity-key-001").await;
    let _permits = h
        .app
        .provider_slots
        .clone()
        .acquire_many_owned(16)
        .await
        .unwrap();
    let (code, value) = h.execute(&p).await;
    assert_eq!(code, 503);
    assert_eq!(value["error"], "connector_busy");
    assert_eq!(h.calls(), 0);
    control_core::lifecycle::stop(&h.app, &h.actor, true)
        .await
        .unwrap();
}
