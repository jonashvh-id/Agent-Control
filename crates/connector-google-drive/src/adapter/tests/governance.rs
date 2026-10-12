use super::*;
use control_core::{
    App,
    models::{Config as RuntimeConfig, Human, Policy, Proposal},
};
#[tokio::test]
async fn actual_drive_requires_exact_approval_and_obeys_revocation() {
    let mut f = fixture(200, "text/plain", b"untrusted content".to_vec()).await;
    let substitute = Drive::new(f.adapter.config.clone()).unwrap();
    let connector = std::mem::replace(&mut f.adapter, substitute);
    let mut config = RuntimeConfig::local(f.temp.path().join("runtime"));
    config.execution_enabled = true;
    let app = App::new(config, vec![Arc::new(connector)]).await.unwrap();
    let id = control_core::identity::create_admin(&app, "admin", "Foundation9!Secure", "admin")
        .await
        .unwrap();
    let actor = Human {
        id,
        username: "admin".into(),
        role: "admin".into(),
        csrf: "".into(),
    };
    let agent = control_core::agents::create(
        &app,
        &actor,
        control_core::agents::Create {
            name: "Drive pilot".into(),
            description: "".into(),
            provider: "".into(),
            runtime: "".into(),
            scopes: vec!["drive:read".into()],
        },
    )
    .await
    .unwrap();
    let aid = agent["id"].as_str().unwrap();
    let credential = control_core::agents::issue(&app, &actor, aid, vec!["drive:read".into()])
        .await
        .unwrap();
    let token = credential["bearer_token"].as_str().unwrap();
    control_core::lifecycle::connector(&app, &actor, "google-drive", true)
        .await
        .unwrap();
    control_core::policy::set(
        &app,
        &actor,
        control_core::policy::Set {
            agent_id: aid.into(),
            connector: "google-drive".into(),
            tool: "files.read".into(),
            mode: Policy::Ask,
        },
    )
    .await
    .unwrap();
    let mut p:Proposal=serde_json::from_value(json!({"connector":"google-drive","tool":"files.read","parameters":read(),"idempotency_key":"drive-approval-001"})).unwrap();
    let pending = control_core::gateway::execute(&app, token, p.clone())
        .await
        .unwrap();
    assert_eq!(pending.status, 202);
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    let approval = pending.body["approval_id"].as_str().unwrap();
    let listed = control_core::approvals::list(&app).await.unwrap();
    assert_eq!(listed[0]["input_preview"]["file_id"], ID);
    assert_eq!(listed[0]["input_preview"]["max_chars"], 4);
    control_core::approvals::decide(&app, &actor, approval, "approved")
        .await
        .unwrap();
    p.approval_id = Some(approval.into());
    let mut changed = p.clone();
    changed.parameters["max_chars"] = json!(5);
    assert_eq!(
        control_core::gateway::execute(&app, token, changed)
            .await
            .unwrap()
            .status,
        403
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        control_core::gateway::execute(&app, token, p.clone())
            .await
            .unwrap()
            .status,
        200
    );
    assert_eq!(
        control_core::gateway::execute(&app, token, p.clone())
            .await
            .unwrap()
            .body["status"],
        "already_executed"
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 2);
    control_core::agents::revoke(&app, &actor, aid, credential["id"].as_str().unwrap())
        .await
        .unwrap();
    assert_eq!(
        control_core::gateway::execute(&app, token, p)
            .await
            .unwrap()
            .status,
        401
    );
    let audit = control_core::audit::list(&app.db.pool)
        .await
        .unwrap()
        .to_string();
    for raw in [token, "untrusted content", ID] {
        assert!(!audit.contains(raw));
    }
}
