# Security boundaries — Rust foundation

Humans authenticate through Argon2id passwords and private hashed sessions. HttpOnly/SameSite cookies, HTTPS secure-cookie configuration, same-origin login and constant-time CSRF verification protect administrative mutations. Agents use separate scoped credentials whose keyed hashes are stored in SQLite. Human cookies and MCP transport state cannot confer agent access; agent tokens cannot confer human roles.

Every MCP request revalidates the current credential and active agent. Discovery, admission and final dispatch use one current-authorization path: identity, scope intersection, policy, connector state, global stop and execution enablement. Tools default DENY. GitHub creation forces ASK; repository deletion has no executable capability. REST follows the same gateway.

Immutable approvals bind agent/credential/tool/connector, keyed exact-parameter fingerprints and idempotency keys. Previews use authenticated AES-GCM with separate-purpose key material and are exposed only to authenticated humans. Current approval status, expiration and enabled approver are checked at consumption and dispatch.

The SQLite ledger durably records claims and dispatch before I/O. Unique keys and conditional state transitions prevent concurrent duplicate dispatch. Uncertain or failed claims are never automatically retried; recovery preserves uncertain state. Already admitted calls can finish after revocation. No exactly-once or cancellation guarantee is made.

GitHub uses a fixed HTTPS origin, finite allowlists, native permissions, no redirects/proxies/retries, bounded responses/time and safe minimal DTOs/receipts. Private owned nonsymlink credentials are reread at dispatch and never given to agents. Local gh-demo references an existing credential in memory only for the designated test repository/loopback; native token rights may be broader.

Runtime paths require a distinct Rust ownership marker and private Unix files; another process cannot open the same runtime. Existing Python/customer directories are rejected. Fresh SQLx migrations do not migrate live data. Database operators remain trusted: local append-only/immutable triggers are not an external tamper-proof audit system.

Input/body/JSON-complexity/result sizes, request/provider deadlines and password/provider concurrency are bounded. Errors omit inputs/provider exception text; audit retains actor/correlation identifiers, policy decisions, keyed fingerprints and minimal receipts, without raw arguments or arbitrary results. Unknown agent-supplied tool names are omitted. Provider content remains untrusted data and cannot dispatch actions.

Only connections routed through this platform are controlled. MCP OAuth, SSO/MFA, managed vaults, tenancy, external audit integrity, reconciliation/cancellation and production security/load certification remain future work. Validation is performed by the implementer, not an independent auditor. See [SECURITY_PARITY.md](SECURITY_PARITY.md) for invariant-by-invariant evidence and [MIGRATION_VALIDATION.md](MIGRATION_VALIDATION.md) for actual checks.
