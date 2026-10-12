# Rust module map

Four crates form one executable. Dependency direction is `server → connector-github → control-core`, `server → connector-google-drive → control-core`, with `server → control-core`. The core depends on a provider trait and never imports HTTP routing, provider implementations or frontend code. Cargo enforces the acyclic crate graph. Phase 3 adds a second adapter without changing the general architecture.

| Module | Responsibility and public interface |
|---|---|
| `control-core/lib.rs` | Compose database/config/trusted provider registry; `App::new`, `App::tool`. No HTTP or direct provider invocation. |
| `models.rs` | Typed Policy and ExecutionState enums; Human/Agent identities; strict Proposal and validated parameter binding. |
| `persistence.rs` | Private owned SQLite runtime, SQLx pool/migrations, single-process lock, transaction acquisition, row DTO conversion. |
| `security/keys.rs` | Library-backed HMAC, authenticated AES-GCM previews, random credentials and constant-time comparison. |
| `security/passwords.rs` | Argon2id password validation/hashing/verification. |
| `security/files.rs` | Owned private regular-file access with O_NOFOLLOW; bounded reads and exclusive creation. |
| `identity/mod.rs`, `identity/repository.rs` | Login/session/bootstrap orchestration separated from SQL. Current enabled humans, rate limiting and bounded password work. |
| `agents/mod.rs`, `agents/repository.rs` | Registry/scoped credential lifecycle separated from SQL; `resolve` computes the current scope intersection. |
| `policy/engine.rs` | Pure effective policy evaluation, default DENY and mandatory confirmation. |
| `policy/mod.rs`, `policy/repository.rs` | Human policy changes/audit and current policy persistence. |
| `approvals/mod.rs`, `approvals/repository.rs` | Encrypted previews, decisions, exact binding/current authority, expiration/one-time consumption; conditional SQL transitions. |
| `execution/mod.rs` | Durable execution repository with typed states and compare-and-swap transitions; unique agent/key claims and crash recovery. |
| `gateway/authorization.rs` | Single current authorization path for discovery, admission and dispatch. |
| `gateway/access.rs` | Human-only effective access diagnostics using that authorization path; no execution and no additional permission grant. |
| `gateway/admission.rs` | Validate tool/schema/provider target, resolve prior claims, create or consume exact approval and claim execution atomically. |
| `gateway/dispatch.rs` | Revalidate authority/binding, commit dispatch marker, bounded external I/O, validate result and persist outcome separately. |
| `gateway/journal.rs` | Minimized governance attribution, known tool identifiers and keyed request fingerprints. |
| `gateway/mod.rs` | Small execution/discovery facade shared by both transports. |
| `audit/` | Structured events, payload-free summaries and append-only SQL persistence. |
| `lifecycle/` | Global stop and agent/connector controls, with SQL repository separate from orchestration. |
| `connector-github/config.rs` | Finite allowlist, excluded ZNode source, bounded timeouts and restricted demo mode. |
| `connector-github/credentials.rs` | Private file reread or fixed bounded local gh command; secret material zeroized and never returned to callers. |
| `connector-github/definitions.rs`, `tools.json` | Literal input/output schemas for the two existing issue operations. No Python generation during build/runtime. |
| `connector-github/adapter.rs` | Fixed-origin REST, explicit retry/redirect/proxy refusal, bounded results, native permissions and safe DTO/receipts. |
| `connector-google-drive/config.rs`, `credentials.rs` | Exact file allowlist and bounded timeout; owned private short-lived token reread, with no credential exposure to the core/model. |
| `connector-google-drive/definitions.rs` | Search/read schemas, scopes and result bounds. No sharing/edit/delete capability. |
| `connector-google-drive/adapter.rs` | Fixed Google API origin, allowlisted metadata/name filtering and bounded text export; native denials, no redirect/proxy/retry and minimal receipts. |
| `server/bootstrap.rs`, `cli.rs`, `config.rs` | Operator command/config validation, composition and listener lifecycle. |
| `server/main.rs`, `lib.rs` | Minimal entry point and router composition. |
| `server/http/` | Thin HTTP handlers, human/agent boundaries, CSRF, sanitized errors, bounded bodies and response headers. |
| `server/http/catalog.rs` | Human-only registered tool catalog for operator forms; does not expose secrets or mark native authentication as verified. |
| `server/mcp.rs` | Official Rust MCP SDK Streamable HTTP and strict arguments mapped into the same gateway. No provider calls. |
| `server/assets.rs` | Embedded frontend with a fixed public asset whitelist. |
| `migrations/0001_foundation.sql` | Fresh Rust schema derived from the Python v1+v2 layout, extended execution states and immutable/append-only triggers. |

`Connector` is the only provider interface: definitions, validation, preview, asynchronous execution and minimized receipt. Adapters receive validated parameters after central admission; routing cannot invoke an adapter directly. The registry accepts only code-owned adapters composed by the server.

Repository functions accept SQLx connections explicitly, allowing domain services to include writes and audit events in the same transaction. No transaction crosses external I/O. Most production files are below 300 lines; modules group coherent responsibilities rather than creating an empty crate for every feature. Tests instantiate domains separately and exercise the full HTTP/MCP composition as well.

Historical Python paths are in `reference/python/`; they are not dependencies of these crates.

Frontend responsibilities remain separate: `features/agents.js` registry/effective access, `features/connect.js` scoped Codex setup, `features/policies.js` permissions, `features/approvals.js` exact human review, `features/google-setup.js` operator guidance, and `services/explanations.js` presentation of reasons. `scripts/codex_pilot.py` is optional model validation tooling, never application runtime.
