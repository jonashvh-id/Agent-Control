# MCP / GitHub contract

The official Rust SDK provides stateless Streamable HTTP at `/mcp/` (also `/mcp`). Every HTTP request requires one current Agent Control bearer token; cookies and session identifiers do not grant agent access. Host/Origin, body and schema boundaries apply before governed execution. Both Rust-native and Python SDK clients are validated against the transport.

`tools/list` returns only currently accessible tools. The two registered names are `github.issues.list` and `github.issues.create`; definition schemas enumerate operator-configured repositories. MCP arguments wrap the provider parameters:

```json
{
  "parameters": {"repository": "jonashvh-id/Agent-Control", "title": "Exact title", "body": "Exact body"},
  "idempotency_key": "unique-create-request-001"
}
```

Creation always returns `requires_approval` with an approval ID before any provider write. A separate human session reads the encrypted-preview-backed proposal at `/api/v1/approvals`, then submits `{"decision":"approved"}` to `/api/v1/approvals/{id}/decision` with CSRF. The agent repeats the identical arguments and adds `approval_id`. Modified parameters/credential/key, expired or consumed grants, tightened policies and revoked authority are rejected.

Listing accepts repository plus optional state (open/closed/all), integer limit 1–20 and page 1–100. Creation accepts a nonblank title up to 256 characters and body up to 12,000, within the additional byte/request bounds. Additional properties and arbitrary URLs are rejected. Results include a safe issue DTO, trust marker and minimized receipt. Repeats return `already_executed` or `execution_uncertain`, never a second provider write.

REST `/api/v1/gateway/execute` uses the same proposal plus `connector:"github"` and `tool:"issues.create"` or `"issues.list"`. HTTP outcomes are included in MCP structured content as `http_status`; failures are tool errors. No MCP approval authority or deletion tool exists.

Provider origin is fixed to `https://api.github.com`, API version `2026-03-10`, with native permission checks, no redirects/proxies/retries and bounded I/O. Routine validation uses isolated fixtures, not live issue writes. Reference documentation: [official Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk), [GitHub REST issues](https://docs.github.com/en/rest/issues/issues), [GitHub API versioning](https://docs.github.com/en/rest/about-the-rest-api/api-versions).
