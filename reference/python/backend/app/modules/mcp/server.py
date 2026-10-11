"""Official MCP SDK Streamable HTTP transport; all execution passes through the gateway."""

import json
from functools import partial

import anyio
from pydantic import ValidationError

from backend.app.core.errors.domain import DomainError
from mcp import types
from mcp.server.lowlevel import Server
from mcp.server.transport_security import TransportSecuritySettings

from .authentication import MCPAuthentication, token_from
from .contracts import mcp_tool, proposal_for


def build_server(get_services, origin):
    async def list_tools(context, params):
        token = token_from(context.request.headers)
        tools = await anyio.to_thread.run_sync(partial(get_services().gateway.discover, token))
        return types.ListToolsResult(tools=[mcp_tool(tool) for tool in tools])

    async def call_tool(context, params):
        try:
            token = token_from(context.request.headers)
            proposal = proposal_for(params.name, params.arguments)
            outcome = await anyio.to_thread.run_sync(
                partial(get_services().gateway.execute, token, proposal), abandon_on_cancel=False
            )
            result = {**outcome.body, "http_status": outcome.status_code}
            return types.CallToolResult(
                content=[types.TextContent(type="text", text=json.dumps(result))],
                structuredContent=result,
                isError=outcome.status_code >= 400,
            )
        except (ValidationError, DomainError):
            return types.CallToolResult(
                content=[types.TextContent(type="text", text="invalid_or_unauthorized_request")],
                structuredContent={"error": "invalid_or_unauthorized_request"},
                isError=True,
            )
        except Exception:
            # Never let the SDK serialize arbitrary adapter/credential exception text.
            return types.CallToolResult(
                content=[types.TextContent(type="text", text="gateway_failed")],
                structuredContent={"error": "gateway_failed"},
                isError=True,
            )

    server = Server("Agent Control", version="0.2.0", on_list_tools=list_tools, on_call_tool=call_tool)
    from urllib.parse import urlsplit

    host = urlsplit(origin).netloc
    transport = server.streamable_http_app(
        streamable_http_path="/",
        stateless_http=True,
        json_response=True,
        max_request_body_size=32768,
        transport_security=TransportSecuritySettings(
            enable_dns_rebinding_protection=True, allowed_hosts=[host], allowed_origins=[origin]
        ),
    )
    return transport, MCPAuthentication(transport, get_services, origin)
