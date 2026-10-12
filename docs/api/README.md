# API contracts

Rust serves the existing `/api/v1` routes: human login/me/logout; agent registry/details/status and scoped credential issue/revoke; policies; human approval decisions/previews; minimized audit; connector enablement; settings; global stop; agent tool discovery/execution. `/health`, `/`, `/ui/*` and governed MCP `/mcp/` are also available.

Humans use `ac_session` cookies and `X-CSRF-Token` for mutations, plus `X-Control-Request: 1` at login. Agent endpoints require their own bearer credential. JSON inputs reject unknown fields and errors omit submitted values. GitHub-native credentials are only read by the adapter, never supplied by an agent request.

Phase 3 adds authenticated human `GET /api/v1/catalog` (registered tools, configured MCP URL and execution flags), administrator-only `GET /api/v1/agents/{id}/access` (effective policies/reasons/credential IDs) and admin+CSRF `PATCH /api/v1/agents/{id}/scopes` (`{"scopes":["drive:read"]}`, 204). Agent credential scope intersection remains authoritative; updating assigned scopes does not update an issued credential. Approval and audit responses join current actor display names for authorized humans without storing those names in audit payloads. The separate [Google Drive contract](MCP_GOOGLE_DRIVE.md) describes its governed MCP tools.

See [MCP_GITHUB.md](MCP_GITHUB.md) and [the migration report](../../RUST_MIGRATION_REPORT.md). `openapi.json` is the archived Python Phase 2 specification, retained as a comparison artifact; it is not a newly generated Rust OpenAPI schema. Essential paths/contracts are exercised against the pinned reference. Rust adds execution-state metadata and normalizes credential-detail scopes to arrays.
