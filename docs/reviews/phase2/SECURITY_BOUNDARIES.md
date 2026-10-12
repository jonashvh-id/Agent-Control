# Security boundaries — GitHub/MCP pilot

Controls cover only connections routed through Agent Control. Direct agent-to-GitHub credentials or connections, filesystem access and provider-side activity outside the gateway remain outside this boundary. The pilot is not an enterprise security certification.

## Identity and authorization

Human and agent identities remain distinct. Opaque human sessions and agent bearer credentials are stored as keyed hashes. Human mutations require role checks, current enabled identity, CSRF and same-origin validation. Every MCP HTTP request requires a current agent bearer credential; cookies, supplied human identities and fabricated session headers grant no agent privileges.

REST and MCP share the same current-authorization service and execution facade. Required scopes must be present in both agent and credential scope sets. A tool must have an explicit policy; unknown/unconfigured tools deny. Global stop, paused/disabled agents, revoked credentials, disabled connectors and execution configuration block later admissions.

Only issues.list and issues.create are registered. GitHub create has mandatory ASK semantics, even with ALLOW. Repository deletion is unavailable and cannot be introduced by modifying arguments or a policy. MCP arguments reject connector/tool authority overrides, extra fields, nonfinite JSON, excessive size/depth and agent-supplied credentials.

## GitHub native boundary and secrets

The adapter calls only `https://api.github.com`, fixed issue endpoints and a finite operator repository allowlist. It refuses ZNode configuration, arbitrary URLs, traversal and unsupported operations. TLS verification stays enabled; redirects, environment proxies and automatic write retries are disabled. Provider responses are bounded and schema-checked. A native GitHub 401/403/404 is a refusal even after platform approval.

Use a fine-grained token restricted to the allowed repository, with Issues read/write and GitHub's required metadata permission. Tokens are read at dispatch from a private, operator-owned, nonsymlink file; missing/revoked files fail closed. Tokens and native exception text are not returned or logged. Token expiry/rotation are controlled by GitHub and the operator file; no provider credential is stored in the database.

`gh-demo` is a local demonstration exception, restricted to `jonashvh-id/Agent-Control` and a loopback origin. It reads only the existing GitHub credential into memory using a fixed CLI command. Its native permissions can be broader than the adapter's allowlist; it is not an enterprise least-privilege credential or managed vault. It never enumerates or copies other credentials. Use the dedicated-token mode for an operational deployment.

## Approval enforcement

The grant binds the exact agent, credential, repository-bearing parameters, connector, tool and idempotency key. Binding columns and encrypted preview are immutable under ordinary SQL updates. Human reviewers see the exact target, title and body as inert text; previews are stored encrypted, not in the audit ledger. Agent transports only receive an approval identifier and status.

Only authenticated human admin/reviewer management sessions decide pending grants. A decision alone is not a permanent permission. Execution rechecks the grant's expiry, one-time consumption, exact fingerprint, credential ownership and enabled approving human alongside current policies and lifecycle. Policy tightening or revocation before final dispatch admission prevents provider I/O. Agent transport cannot approve its own request.

## Dispatch, revocation and idempotency

Prepared claim and final dispatch admission commit before provider I/O. The dispatch marker is the linearization point: controls changed after that point block subsequent admissions but cannot undo the already admitted operation. No database transaction is held while waiting on GitHub, so revocation/global-stop updates are not blocked by network I/O. Cancellation of already admitted requests remains unimplemented.

Keys are scoped to agent identity; changed parameters with a reused key fail. Approval replay and concurrent duplicate execution cannot produce a second dispatch. A timeout, native failure, crash or uncertain write is not retried automatically. Exactly-once delivery is not promised; reconciliation after uncertain writes is still needed. Results are untrusted data, never execution instructions.

## Audit

Persist actor/agent IDs, operation, decision, approval/execution identifiers, UTC timestamps, correlation IDs, keyed input digest, parameter shape and minimized result receipt. GitHub number/URL are retained to identify the external action. Raw issue titles/bodies, credentials, raw provider payloads and exception messages are absent. Authorized previews are separate encrypted approval data.

The ledger remains append-only under SQLite triggers and survives disable/revoke operations. A filesystem/database administrator can still alter the file or triggers; external tamper evidence and retention/export remain planned. Failed REST and MCP bearer checks record a denial with verified stored credential/agent identifiers when available, without storing submitted bearer values. Full distributed anti-abuse and malformed-transport instrumentation remain incomplete.

## Remaining limits

Preprovisioned bearer MCP access is implemented; OAuth discovery/token flows and SSO/MFA are not. Runtime key custody is a local mode-0600 file, not a managed vault or database encryption. There is no tenant isolation, arbitrary enterprise application coverage, prompt-injection immunity, long-running cancellation, provider reconciliation service, load/penetration certification or production public deployment.

The default execution flag is false and every newly registered connector is disabled. The live pilot uses a new isolated development operator/runtime and a separate external client; its real GitHub write requires an explicit human decision on the exact proposed content. No ZNode source, Git state, database or deployment is changed.
