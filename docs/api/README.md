# API contracts

Rust serves the existing `/api/v1` routes: human login/me/logout; agent registry/details/status and scoped credential issue/revoke; policies; human approval decisions/previews; minimized audit; connector enablement; settings; global stop; agent tool discovery/execution. `/health`, `/`, `/ui/*` and governed MCP `/mcp/` are also available.

Humans use `ac_session` cookies and `X-CSRF-Token` for mutations, plus `X-Control-Request: 1` at login. Agent endpoints require their own bearer credential. JSON inputs reject unknown fields and errors omit submitted values. GitHub-native credentials are only read by the adapter, never supplied by an agent request.

See [MCP_GITHUB.md](MCP_GITHUB.md) and [the migration report](../../RUST_MIGRATION_REPORT.md). `openapi.json` is the archived Python Phase 2 specification, retained as a comparison artifact; it is not a newly generated Rust OpenAPI schema. Essential paths/contracts are exercised against the pinned reference. Rust adds execution-state metadata and normalizes credential-detail scopes to arrays.
