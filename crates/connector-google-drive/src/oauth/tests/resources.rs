use super::*;
use control_core::{
    connector::Connector,
    models::{Policy, Proposal},
    policy::Set,
};
#[tokio::test]
async fn native_reader_extent_must_be_complete_and_not_broader() {
    let f = fixture().await;
    f.connect().await;
    f.roots().await;
    f.native.mode.store(1, Ordering::SeqCst);
    let v = f.manager.verify_extent().await.unwrap();
    assert_eq!(v["verified"], false);
    assert_eq!(v["outside_objects"], 1);
    f.native.mode.store(2, Ordering::SeqCst);
    assert_eq!(
        f.manager.verify_extent().await.err().unwrap().code,
        "google_reader_folder_required"
    );
    f.native.mode.store(3, Ordering::SeqCst);
    assert_eq!(
        f.manager.verify_extent().await.err().unwrap().code,
        "google_native_extent_incomplete"
    );
    assert_eq!(
        f.manager.status().await.unwrap()["native_verification"]["verified"],
        false
    );
}
#[tokio::test]
async fn policy_and_resource_binding_protect_content_and_encrypted_observations() {
    let f = fixture().await;
    f.connect().await;
    let (root, other) = f.roots().await;
    let agent = control_core::agents::create(
        &f.app,
        &f.actor,
        control_core::agents::Create {
            name: "Resource agent".into(),
            description: "".into(),
            provider: "".into(),
            runtime: "Codex".into(),
            scopes: vec!["drive:read".into()],
        },
    )
    .await
    .unwrap();
    let aid = agent["id"].as_str().unwrap();
    let cred = control_core::agents::issue(&f.app, &f.actor, aid, vec!["drive:read".into()])
        .await
        .unwrap();
    let token = cred["bearer_token"].as_str().unwrap();
    control_core::lifecycle::connector(&f.app, &f.actor, "google-drive", true)
        .await
        .unwrap();
    control_core::policy::set(
        &f.app,
        &f.actor,
        Set {
            agent_id: aid.into(),
            connector: "google-drive".into(),
            tool: "files.read".into(),
            mode: Policy::Allow,
        },
    )
    .await
    .unwrap();
    let mut p:Proposal=serde_json::from_value(json!({"connector":"google-drive","tool":"files.read","parameters":{"resource_id":root,"file_id":DOC,"max_chars":20},"idempotency_key":"resource-read-001"})).unwrap();
    let before = f.native.calls.load(Ordering::SeqCst);
    assert_eq!(
        control_core::gateway::execute(&f.app, token, p.clone())
            .await
            .unwrap()
            .status,
        403
    );
    assert_eq!(f.native.calls.load(Ordering::SeqCst), before);
    control_core::resources::policy::set(&f.app, &f.actor, aid, &root, Policy::Ask)
        .await
        .unwrap();
    let defs = control_core::gateway::discover(&f.app, token)
        .await
        .unwrap();
    assert_eq!(
        defs[0]["input_schema"]["properties"]["resource_id"]["enum"],
        json!([root])
    );
    let pending = control_core::gateway::execute(&f.app, token, p.clone())
        .await
        .unwrap();
    assert_eq!(pending.status, 202);
    assert_eq!(f.native.exports.load(Ordering::SeqCst), 0);
    let approval = pending.body["approval_id"].as_str().unwrap();
    control_core::approvals::decide(&f.app, &f.actor, approval, "approved")
        .await
        .unwrap();
    p.approval_id = Some(approval.into());
    let mut changed = p.clone();
    changed.parameters["resource_id"] = json!(other);
    assert_eq!(
        control_core::gateway::execute(&f.app, token, changed)
            .await
            .unwrap()
            .status,
        403
    );
    assert_eq!(
        control_core::gateway::execute(&f.app, token, p.clone())
            .await
            .unwrap()
            .status,
        200
    );
    assert_eq!(
        control_core::gateway::execute(&f.app, token, p.clone())
            .await
            .unwrap()
            .body["status"],
        "already_executed"
    );
    assert_eq!(f.native.exports.load(Ordering::SeqCst), 1);
    let raw = control_core::audit::list(&f.app.db.pool)
        .await
        .unwrap()
        .to_string();
    for value in [DOC, "Fixture document", "Untrusted fixture document", token] {
        assert!(!raw.contains(value));
    }
    let dashboard = control_core::operations::dashboard(&f.app).await.unwrap();
    assert!(dashboard.to_string().contains("Fixture document"));
    assert!(!dashboard.to_string().contains("Untrusted fixture document"));
    control_core::resources::policy::set(&f.app, &f.actor, aid, &root, Policy::Deny)
        .await
        .unwrap();
    assert_eq!(
        control_core::gateway::execute(&f.app, token, p)
            .await
            .unwrap()
            .status,
        403
    );
    assert_eq!(f.native.exports.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn nested_resource_cannot_be_bypassed_using_its_parent_or_folder_id() {
    let f = fixture().await;
    f.connect().await;
    let (root, _) = f.roots().await;
    f.root(CHILD).await;
    f.native.mode.store(4, Ordering::SeqCst);
    f.manager.verify_extent().await.unwrap();
    let provider = crate::managed::ManagedDrive::new(f.manager.clone());
    assert_eq!(
        provider
            .execute(
                "files.read",
                &json!({"resource_id":root,"file_id":DOC}),
                "fixture"
            )
            .await
            .err()
            .unwrap()
            .code,
        "google_use_specific_resource"
    );
    assert_eq!(
        provider
            .execute(
                "files.search",
                &json!({"resource_id":root,"folder_id":CHILD}),
                "fixture"
            )
            .await
            .err()
            .unwrap()
            .code,
        "google_use_specific_resource"
    );
    assert_eq!(f.native.exports.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn resource_changes_cannot_commit_stale_native_verification() {
    let f = fixture().await;
    f.connect().await;
    let (generation, revision) = f.manager.store.revision("google-drive").await.unwrap();
    f.root(ROOT).await;
    assert!(
        f.manager
            .store
            .verification_checked(
                "google-drive",
                &json!({"verified":true}),
                generation,
                revision
            )
            .await
            .is_err()
    );
}
