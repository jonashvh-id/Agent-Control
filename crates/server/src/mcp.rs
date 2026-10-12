use axum::{Router, http::request::Parts};
use control_core::{App, gateway, models::Proposal};
use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool, ToolAnnotations,
    },
    service::RequestContext,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
#[derive(Clone)]
struct Handler(Arc<App>);
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    parameters: serde_json::Map<String, Value>,
    idempotency_key: String,
    approval_id: Option<String>,
}
fn token(context: &RequestContext<RoleServer>) -> Result<String, ErrorData> {
    let parts = context
        .extensions
        .get::<Parts>()
        .ok_or_else(|| ErrorData::invalid_request("authentication_required", None))?;
    crate::http::auth::bearer(&parts.headers)
        .map_err(|_| ErrorData::invalid_request("authentication_required", None))
}
impl ServerHandler for Handler {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_server_info(
            rmcp::model::Implementation::new("Agent Control", env!("CARGO_PKG_VERSION")),
        )
    }
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let token = token(&context)?;
        let list = gateway::discover(&self.0, &token)
            .await
            .map_err(|_| ErrorData::invalid_request("unauthorized_request", None))?;
        let tools=list.into_iter().map(|tool| {
            let schema=json!({"type":"object","properties":{"parameters":tool["input_schema"],"idempotency_key":{"type":"string","minLength":8,"maxLength":128,"pattern":"^[A-Za-z0-9_-]+$"},"approval_id":{"type":"string","pattern":"^[a-f0-9]{32}$"}},"required":["parameters","idempotency_key"],"additionalProperties":false});
            Tool::new(format!("{}.{}",tool["connector"].as_str().unwrap(),tool["name"].as_str().unwrap()),tool["description"].as_str().unwrap().to_string(),schema.as_object().unwrap().clone()).with_annotations(ToolAnnotations::new().read_only(matches!(tool["name"].as_str(),Some("issues.list"|"files.search"|"files.read"))).open_world(true))
        }).collect();
        {
            Ok(ListToolsResult {
                tools,
                ..Default::default()
            })
        }
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let token = token(&context)?;
        let result = async {
            let args: Arguments =
                serde_json::from_value(json!(request.arguments.unwrap_or_default()))
                    .map_err(|_| control_core::error::Error::new("invalid_request", 422))?;
            let (connector, tool) = request
                .name
                .split_once('.')
                .unwrap_or(("unknown", "unknown"));
            let p = Proposal {
                connector: connector.into(),
                tool: tool.into(),
                parameters: args.parameters,
                idempotency_key: args.idempotency_key,
                approval_id: args.approval_id,
            };
            gateway::execute(&self.0, &token, p).await
        }
        .await;
        let (mut body, status) = match result {
            Ok(r) => (r.body, r.status),
            Err(e) => (json!({"error":e.code}), e.status),
        };
        body["http_status"] = json!(status);
        let mut result = if status >= 400 {
            CallToolResult::error(vec![ContentBlock::text(body.to_string())])
        } else {
            CallToolResult::success(vec![ContentBlock::text(body.to_string())])
        };
        result.structured_content = Some(body);
        Ok(result.into())
    }
}
pub fn router(app: Arc<App>) -> Router<Arc<App>> {
    let url = url::Url::parse(&app.config.origin).expect("validated origin");
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true);
    let mut config = config;
    config.allowed_hosts = vec![url[url::Position::BeforeHost..url::Position::AfterPort].into()];
    config.allowed_origins = vec![app.config.origin.clone()];
    config.max_request_body_bytes = 32768;
    let service = StreamableHttpService::new(
        move || Ok(Handler(app.clone())),
        Arc::new(LocalSessionManager::default()),
        config,
    );
    Router::new()
        .route_service("/mcp/", service.clone())
        .route_service("/mcp", service)
}
