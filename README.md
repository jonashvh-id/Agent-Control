# Agent Control

A standalone Rust platform for governing external AI agents through MCP and REST. Human administration, scoped agent identities, default-deny policies, exact approvals, durable execution records and minimized audit share one governance engine. GitHub and Google Drive adapters share this engine; the JavaScript interface shows effective access and explains blocked requests.

## Build and run

Rust 1.89+ and a C toolchain for bundled SQLite. The validated runtime is Unix; private-file ownership and permissions are enforced there. Python and Node.js are not runtime dependencies. Static frontend assets and SQL migrations are embedded in the executable.

```sh
cargo build --release --locked
export AGENT_CONTROL_DATA_DIR=/absolute/private/path/new-agent-control-rust-runtime
./target/release/agent-control-server create-admin --username admin
./target/release/agent-control-server serve --port 8010
```

Open http://127.0.0.1:8010. Passwords are entered through the terminal without echo. No default account is created. The data directory must be empty on first use and remains private; only one process may open it. Existing Python/customer directories are rejected. The Python baseline is archived in `reference/python/` and Git history.

The server binds to loopback. For a TLS reverse proxy, set `AGENT_CONTROL_ORIGIN` to the public HTTPS origin, `AGENT_CONTROL_SECURE_COOKIES=true`, and keep `serve --port` as the private listener port. Forward the original Host; proxy-provided identity/address headers are not trusted. Environment examples are documentation and are not automatically loaded.

## GitHub and MCP

Save a dedicated fine-grained GitHub token in an operator-owned nonsymlink mode-0600 file outside source control, scoped to the designated repository and Issues read/write plus required metadata access.

```sh
export AGENT_CONTROL_GITHUB_REPOSITORIES=jonashvh-id/Agent-Control
export AGENT_CONTROL_GITHUB_CREDENTIAL_SOURCE=file
export AGENT_CONTROL_GITHUB_TOKEN_FILE=/absolute/private/path/github.token
export AGENT_CONTROL_EXECUTION_ENABLED=true
./target/release/agent-control-server serve
```

The configured connector starts disabled. Enable it in Applications; register an agent, choose capabilities and use Connect Codex to issue a scoped credential. In Permissions, set `github / issues.list` to ALLOW and `github / issues.create` to ASK. Creating an issue always requires approval, including when its explicit policy is ALLOW. Repository deletion has no executable tool. Native GitHub permissions remain authoritative.

Agents connect to `/mcp/` with their Agent Control bearer token, or use `/api/v1/gateway/tools` and `/api/v1/gateway/execute`. MCP tool names are `github.issues.list` and `github.issues.create`. Arguments contain `parameters`, `idempotency_key`, and optional `approval_id`. Only a separately authenticated human session may approve the exact proposal. See [the API contract](docs/api/MCP_GITHUB.md).

Local `gh-demo` can read the existing gh keychain credential in memory, exclusively for `jonashvh-id/Agent-Control` and a loopback origin. Its native rights may be broader; the dedicated file credential is the operational path. Neither mode exposes the GitHub credential to an agent or persists it in SQLite.

An idempotency key binds one request. Dispatched or uncertain writes are never retried automatically; a repeat returns a minimized receipt or a refusal. Revocation blocks new admissions, while an already admitted provider call may finish. This does not provide an exactly-once provider guarantee.

## Google Drive: integrated OAuth and folder permissions

Register a Web OAuth client in your own Google Cloud project, enable Drive API, and register the exact callback URI shown by **Applications → Configure Google OAuth**. Use a dedicated Google identity shared as **Reader** only on the intended folders. No Google access token is copied into the dashboard or supplied to an agent.

```sh
export AGENT_CONTROL_GOOGLE_OAUTH_CLIENT_ID=YOUR_CLIENT.apps.googleusercontent.com
export AGENT_CONTROL_GOOGLE_OAUTH_SECRET_FILE=/absolute/private/path/google-client.secret
```

The secret file contains the OAuth client secret only, owned by the server operator, mode 0600, outside source control. Start the server with these variables and the execution flag when needed. From Applications, connect the dedicated reader account; in Resources, select its folders and verify the native perimeter. Grant each agent explicit folder permissions and tool permissions. The stricter DENY / ASK / ALLOW rule applies; a tool grant alone confers no folder access. Enable the application separately.

Search returns bounded direct-child metadata; reading accepts Google Docs/plain text up to 6,000 characters. Each request includes an Agent Control `resource_id`. Discovery lists only the agent’s permitted resources. A configured nested folder is a separate boundary; its permission cannot be bypassed through a parent. Exact approvals include folder/document parameters. Operations shows observed requests, data returned, refusals and intervention counts without claiming persistent MCP online presence.

Google has no read-only OAuth scope scoped to folder IDs. The server requests only `drive.readonly`; native resource restriction comes from the dedicated identity’s Google sharing. A bounded inventory and current folder ancestry are checked before each managed execution. These checks are snapshots, not proof of all ACLs or immediate cancellation of in-flight requests. Disabling a folder here does not change its native Google ACL.

See [OAuth configuration and resource contract](docs/PHASE4_OAUTH_RESOURCES.md). The previous manually configured file-token adapter remains an explicit compatibility mode; it cannot be configured alongside integrated OAuth. The historical [Phase 3 pilot](docs/PHASE3.md) used that adapter. The new integrated OAuth flow has been validated against isolated providers; a live connection awaits the user’s Google Cloud client and dedicated reader identity.

## Validate

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
(cd frontend && npm run check)
```

Routine tests use isolated providers. The suite includes an independent native MCP SDK client and a copied executable starting outside the workspace with an empty PATH. Optional cross-language tooling compares the pinned Python baseline and verifies a Python SDK client against Rust; see [MIGRATION_VALIDATION.md](MIGRATION_VALIDATION.md). Do not create live GitHub issues for routine validation.

Read [RUST_MIGRATION_REPORT.md](RUST_MIGRATION_REPORT.md), [RUST_MODULE_MAP.md](RUST_MODULE_MAP.md), [SECURITY_PARITY.md](SECURITY_PARITY.md), [ARCHITECTURE.md](ARCHITECTURE.md) and [SECURITY_BOUNDARIES.md](SECURITY_BOUNDARIES.md). Historical extraction and live Phase 2 evidence remain in [EXTRACTION_REPORT.md](EXTRACTION_REPORT.md) and `docs/reviews/`.

OAuth MCP, integrated Google OAuth/refresh, SSO/MFA, managed vaults, tenant isolation, reconciliation/cancellation, external audit integrity and production certification remain future work. MFA, managed secrets and independent review remain prerequisites for client deployment. No deployment or release is performed by this work.

Ubuntu 24.04 CI runs Rust 1.89 formatting, clippy, tests, frontend checks and the standalone release build on pull requests and main pushes. No application secrets or live providers are used. See [.github/workflows/ci.yml](.github/workflows/ci.yml) and [the final migration review](docs/rust/CODE_REVIEW.md).
