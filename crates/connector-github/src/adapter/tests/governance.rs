use super::*;

#[tokio::test]
async fn actual_adapter_respects_domain_approval_and_native_denial() {
    let mut f = fixture(403, "native denial".into(), 0).await;
    let (app, actor, aid, token) = governed(&mut f, "issues:write").await;
    control_core::policy::set(
        &app,
        &actor,
        control_core::policy::Set {
            agent_id: aid,
            connector: "github".into(),
            tool: "issues.create".into(),
            mode: control_core::models::Policy::Allow,
        },
    )
    .await
    .unwrap();
    let mut proposal:control_core::models::Proposal=serde_json::from_value(json!({"connector":"github","tool":"issues.create","parameters":input(),"idempotency_key":"real-adapter-key"})).unwrap();
    let pending = control_core::gateway::execute(&app, &token, proposal.clone())
        .await
        .unwrap();
    assert_eq!(pending.status, 202);
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    let approval = pending.body["approval_id"].as_str().unwrap();
    control_core::approvals::decide(&app, &actor, approval, "approved")
        .await
        .unwrap();
    proposal.approval_id = Some(approval.into());
    let result = control_core::gateway::execute(&app, &token, proposal.clone())
        .await
        .unwrap();
    assert_eq!(result.status, 403);
    assert_eq!(result.body["error"], "github_access_denied");
    assert_eq!(
        control_core::gateway::execute(&app, &token, proposal)
            .await
            .unwrap()
            .status,
        409
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
}
async fn governed(
    f: &mut Fixture,
    scope: &str,
) -> (
    Arc<control_core::App>,
    control_core::models::Human,
    String,
    String,
) {
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
            scopes: vec![scope.into()],
        },
    )
    .await
    .unwrap();
    let aid = agent["id"].as_str().unwrap();
    let credential = control_core::agents::issue(&app, &actor, aid, vec![scope.into()])
        .await
        .unwrap();
    let token = credential["bearer_token"].as_str().unwrap();
    control_core::lifecycle::connector(&app, &actor, "github", true)
        .await
        .unwrap();
    (app, actor, aid.to_string(), token.to_string())
}

#[tokio::test]
async fn approved_read_preview_includes_filters_and_rejects_changes() {
    let mut f = fixture(200, json!([]).to_string(), 0).await;
    let (app, actor, agent, token) = governed(&mut f, "issues:read").await;
    control_core::policy::set(
        &app,
        &actor,
        control_core::policy::Set {
            agent_id: agent,
            connector: "github".into(),
            tool: "issues.list".into(),
            mode: control_core::models::Policy::Ask,
        },
    )
    .await
    .unwrap();
    let mut proposal: control_core::models::Proposal = serde_json::from_value(json!({
        "connector":"github", "tool":"issues.list", "idempotency_key":"read-preview-key",
        "parameters":{"repository":"demo/repo", "state":"closed", "limit":3, "page":2}
    }))
    .unwrap();
    let pending = control_core::gateway::execute(&app, &token, proposal.clone())
        .await
        .unwrap();
    assert_eq!(pending.status, 202);
    let approval = pending.body["approval_id"].as_str().unwrap();
    let review = control_core::approvals::list(&app).await.unwrap();
    let preview = &review[0]["input_preview"];
    assert_eq!(preview["state"], "closed");
    assert_eq!(preview["limit"], 3);
    assert_eq!(preview["page"], 2);
    assert_eq!(preview["repository"], "demo/repo");
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    control_core::approvals::decide(&app, &actor, approval, "approved")
        .await
        .unwrap();
    proposal.approval_id = Some(approval.into());
    let mut changed = proposal.clone();
    changed.parameters["page"] = json!(3);
    assert_eq!(
        control_core::gateway::execute(&app, &token, changed)
            .await
            .unwrap()
            .status,
        403
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        control_core::gateway::execute(&app, &token, proposal)
            .await
            .unwrap()
            .status,
        200
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
}
