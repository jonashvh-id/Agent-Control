mod common;
use common::Harness;
use reqwest::Method;
use rmcp::{
    ServiceExt,
    model::CallToolRequestParams,
    transport::streamable_http_client::{
        StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
    },
};
use serde_json::{Value, json};
#[tokio::test]
async fn independent_sdk_discovers_and_executes_governed_tools() {
    let h = Harness::new().await;
    h.policy("issues.list", "allow").await;
    h.policy("issues.create", "allow").await;
    let config = StreamableHttpClientTransportConfig::with_uri(format!("{}/mcp/", h.origin))
        .auth_header(&h.token);
    let transport = StreamableHttpClientTransport::from_config(config);
    let service = ().serve(transport).await.unwrap();
    let tools = service.list_all_tools().await.unwrap();
    assert_eq!(tools.len(), 2);
    let call = |name: &str, args: Value| {
        serde_json::from_value::<CallToolRequestParams>(json!({"name":name,"arguments":args}))
            .unwrap()
    };
    let read = service
        .call_tool(call(
            "github.issues.list",
            json!({"parameters":{"repository":"demo/repo"},"idempotency_key":"mcp-read-001"}),
        ))
        .await
        .unwrap();
    assert_eq!(read.structured_content.unwrap()["status"], "executed");
    let args = json!({"parameters":{"repository":"demo/repo","title":"MCP exact title","body":"MCP exact body"},"idempotency_key":"mcp-create-001"});
    let pending = service
        .call_tool(call("github.issues.create", args.clone()))
        .await
        .unwrap()
        .structured_content
        .unwrap();
    assert_eq!(pending["status"], "requires_approval");
    assert_eq!(h.calls(), 1);
    let aid = pending["approval_id"].as_str().unwrap();
    let r = h
        .human(Method::POST, &format!("/api/v1/approvals/{aid}/decision"))
        .json(&json!({"decision":"approved"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let mut approved = args.clone();
    approved["approval_id"] = json!(aid);
    let mut changed = approved.clone();
    changed["parameters"]["title"] = json!("changed");
    assert_eq!(
        service
            .call_tool(call("github.issues.create", changed))
            .await
            .unwrap()
            .is_error,
        Some(true)
    );
    assert_eq!(h.calls(), 1);
    let result = service
        .call_tool(call("github.issues.create", approved.clone()))
        .await
        .unwrap();
    assert_eq!(result.structured_content.unwrap()["status"], "executed");
    let replay = service
        .call_tool(call("github.issues.create", approved))
        .await
        .unwrap();
    assert_eq!(
        replay.structured_content.unwrap()["status"],
        "already_executed"
    );
    assert_eq!(h.calls(), 2);
    h.policy("issues.create", "deny").await;
    assert_eq!(service.list_all_tools().await.unwrap().len(), 1);
    let result = service
        .call_tool(call("github.issues.create", args.clone()))
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(true));
    let mut override_args = args;
    override_args["connector"] = json!("override");
    assert_eq!(
        service
            .call_tool(call("github.issues.list", override_args))
            .await
            .unwrap()
            .is_error,
        Some(true)
    );
    control_core::agents::revoke(&h.app, &h.actor, &h.agent, &h.credential)
        .await
        .unwrap();
    assert!(service.list_all_tools().await.is_err());
    service.cancel().await.unwrap();
}
#[tokio::test]
async fn human_cookie_never_authenticates_mcp() {
    let h = Harness::new().await;
    for method in [Method::GET, Method::POST] {
        let r = h
            .human(method, "/mcp/")
            .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 401);
    }
}
#[tokio::test]
async fn mcp_origin_rejected_on_get_and_post() {
    let h = Harness::new().await;
    for method in [Method::GET, Method::POST] {
        let r = h
            .agent_request(method, "/mcp/")
            .header("origin", "https://evil.example")
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 403);
    }
}
#[tokio::test]
async fn malformed_mcp_does_not_dispatch() {
    let h = Harness::new().await;
    let r = h
        .agent_request(Method::POST, "/mcp/")
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .body("{broken")
        .send()
        .await
        .unwrap();
    let status = r.status();
    let body = r.text().await.unwrap();
    assert_eq!(status, 400, "{body}");
    assert_eq!(h.calls(), 0);
}
#[tokio::test]
async fn forged_session_cannot_restore_revoked_identity() {
    let h = Harness::new().await;
    control_core::agents::revoke(&h.app, &h.actor, &h.agent, &h.credential)
        .await
        .unwrap();
    let r = h
        .agent_request(Method::POST, "/mcp/")
        .header("mcp-session-id", "forged")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 401);
    let audit = control_core::audit::list(&h.app.db.pool).await.unwrap();
    assert_eq!(audit[0]["agent_id"], h.agent);
    assert!(!audit.to_string().contains(&h.token));
}
#[tokio::test]
async fn duplicate_authorization_header_rejected() {
    let h = Harness::new().await;
    let r = h
        .agent_request(Method::POST, "/mcp/")
        .header("authorization", "Bearer fake")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 401);
}
