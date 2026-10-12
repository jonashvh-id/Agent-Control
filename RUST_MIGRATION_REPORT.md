# Rust migration report

The migration checkpoint is complete. Subsequent Phase 3 product changes preserve this Rust architecture and are documented separately in [the Phase 3 report](docs/reviews/PHASE3_REPORT.md) and [current module map](RUST_MODULE_MAP.md).

Agent Control's primary backend is a standalone Rust executable. Axum, Tokio, SQLx/SQLite, Serde, Tracing and the official Rust MCP SDK replace the Python/FastAPI application runtime. The existing JavaScript frontend and the two GitHub issue capabilities are retained. Work was prepared on `migration/rust-backend`, based on `b946766e705856131b59f386b5ef657310f7dfdf`; the additive implementation checkpoint is `8cd480d`. The original Git history is preserved; fusion is gated by reviewed local validation and Ubuntu CI.

## Migration and module provenance

| Python reference | Rust destination |
|---|---|
| `backend/app/main.py`, `bootstrap.py`, `cli.py`, core/config | `crates/server/{main,lib,bootstrap,cli,config}.rs`, `http/`, `assets.rs` |
| core/database schema, connection and migration 2 | `control-core/persistence.rs`, `migrations/0001_foundation.sql` |
| core/security password/token/fingerprint/sealed helpers | `control-core/security/{passwords,keys,files}.rs` |
| modules/identity | `control-core/identity/{mod,repository}.rs` |
| modules/agents | `control-core/agents/{mod,repository}.rs` |
| modules/policies | `control-core/policy/{engine,mod,repository}.rs` |
| modules/approvals | `control-core/approvals/{mod,repository}.rs` |
| modules/execution and gateway | `control-core/execution/mod.rs`, `gateway/{mod,authorization,admission,dispatch,journal}.rs` |
| modules/audit and lifecycle | `control-core/audit/`, `lifecycle/` |
| modules/mcp | `crates/server/mcp.rs`, `http/{auth,boundary}.rs` |
| connectors/github | `crates/connector-github/src/` |
| frontend | Retained at `frontend/`, embedded in Rust; only the version badge changes |

Python code has been reimplemented in Rust, rather than wrapped or embedded. Literal GitHub schemas preserve the source contract, and the SQL migration preserves the table lineage while extending execution states. The previous implementation is retained in Git history, not distributed as source in the current tree.

## Design and behavior

Three crates retain explicit boundaries without microservices or empty per-feature crates. Core services depend on a typed Connector interface; routing and the GitHub adapter never own policy decisions. HTTP/MCP share the same gateway. Services and repositories are separated, and SQLite transactions are explicit and never span network I/O. Main remains a small entry point.

Typed execution states distinguish prepared, dispatched, executed, failed and uncertain. A durable dispatch marker precedes external I/O; the unique claim prevents concurrent duplicates. At startup, unfinished records become uncertain under an exclusive runtime lock. GitHub's lack of provider idempotency is explicit, and write retries are disabled both in the ledger and HTTP client.

Passwords use Argon2id rather than scrypt. Approval previews use authenticated AES-GCM rather than Fernet. Rust has its own private binary runtime key and ownership marker. These formats are intentionally separate from the old data; there is no automatic customer-data cutover. Future migration/rollback strategy is in `docs/rust/DATA_MIGRATION.md`.

Compatible behaviors include current human/agent identity boundaries, scoped credentials, default DENY, mandatory issue confirmation, exact immutable approval binding, expiration/one-time consumption, revalidation, lifecycle controls, minimized audit and native GitHub permission enforcement. The existing admin API paths and MCP arguments remain. Version becomes 0.3.0; execution responses add explicit state details, credential detail scopes become JSON arrays, and nonintegral read pagination is rejected rather than silently defaulted. Explicit public origin and private listener port are separate for TLS proxies. Current flags retain case-insensitive true handling. Final review requires the exact configured Host authority on both REST and MCP, and GitHub read previews include effective state/limit/page. See [CODE_REVIEW.md](docs/rust/CODE_REVIEW.md).

## Evidence and limits

The original Python suite was rerun before migration: 65 passed. A pinned-reference comparison verifies 28 equivalent HTTP observations across registration, policies, exact approvals, execution, replay, conflict, controls, revocation and audit attribution. Independent native Rust and separate-process Python SDK clients exercise Streamable HTTP with governed issue reads/creation and revocation against isolated providers. The actual GitHub adapter separately exercises bounded native HTTP responses and the same domain approval path.

The detailed current Rust, frontend, copied-executable, release/CLI/browser, coverage and source-preservation results are in [MIGRATION_VALIDATION.md](MIGRATION_VALIDATION.md). [SECURITY_PARITY.md](SECURITY_PARITY.md) maps every critical invariant to concrete tests. No extra live GitHub issue was created; the existing issue #1 is historical Python Phase 2 evidence, not a Rust live-write claim.

The runtime remains a governance foundation, with bearer provisioning and local private files. OAuth MCP, SSO/MFA, managed vaults, tenancy, reconciliation/cancellation, external audit integrity and independent production/load/security certification remain future work. No public deployment, release publication or live customer-data migration is performed.
