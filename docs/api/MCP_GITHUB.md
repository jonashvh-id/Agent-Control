# MCP and GitHub pilot

Endpoint: `/mcp/`, official SDK Streamable HTTP with JSON responses. Authenticate every request with an Agent Control bearer in the Authorization header. This is preprovisioned internal access, not a complete OAuth authorization server. Browser Origin/Host validation is enabled. Protocol negotiation/JSON-RPC framing are handled by pinned official SDK 2.3.0.

`tools/list` is computed using current agent/credential/scopes/policy/connector/global state. The GitHub tool names are `github.issues.list` and `github.issues.create`. Deleted/unregistered/denied tools cannot dispatch even if called directly.

A creation proposal:

```json
{"parameters":{"repository":"jonashvh-id/Agent-Control","title":"Reviewed title","body":"Reviewed exact body"},"idempotency_key":"stable-client-request-key"}
```

Tool name determines connector and operation. Arguments cannot supply connector/tool/actor/credential overrides. `issues.create` always requires approval; a policy requesting ALLOW is reduced to ASK for that tool. The caller receives `status=requires_approval`, `approval_id` and correlation ID, and no GitHub write occurs.

An authorized human reads the decrypted exact preview at GET `/api/v1/approvals` and POSTs `/api/v1/approvals/{id}/decision` with `approved` or `denied`, using its human session and CSRF. An agent token cannot access that route. After approval, the client repeats the identical arguments plus `approval_id`.

Success returns an untrusted minimized issue result and receipt (number/URL). Duplicate success returns a receipt, not another provider write. Prepared/failed duplicate yields execution_uncertain; changed key parameters, stale/denied/consumed grant or current revocation/policy denial cannot bypass controls. REST `/api/v1/gateway/execute` uses exactly the same facade.

External client example:

```sh
# Set AGENT_CONTROL_AGENT_TOKEN through your secret manager/process environment.
.venv/bin/python scripts/mcp_issue_client.py --url http://127.0.0.1:8010/mcp/ --tool discover
.venv/bin/python scripts/mcp_issue_client.py --url http://127.0.0.1:8010/mcp/ --tool github.issues.create --request-file /private/path/proposal.json
```

The external client does not load GitHub credentials or contact GitHub. References: [official MCP SDK](https://py.sdk.modelcontextprotocol.io/api/mcp/server/lowlevel/server/), [GitHub issue permissions and API](https://docs.github.com/en/rest/issues/issues?apiVersion=2026-03-10).
