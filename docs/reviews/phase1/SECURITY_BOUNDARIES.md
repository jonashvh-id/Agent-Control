# Security boundaries

This release is a foundation. It governs only requests routed through its gateway. It cannot constrain direct agent-to-provider connections, prompt behavior, external credentials held by agents or actions outside the gateway.

## Agent identity

Agents have their own registry and opaque bearer credentials, not human user accounts or roles. Random bearer values are shown once; only keyed HMAC hashes persist. Authentication reads credential status and agent state from the database. Effective scopes are the intersection of credential and agent scopes. Revocation is immediate for later server admissions, not a UI flag.

## Human identity

Humans authenticate by username/password using extracted ZNode scrypt functions. Opaque human sessions are HMAC-hashed server-side, finite-lived and checked against current human enabled state. Cookies are HttpOnly and SameSite Strict, with Secure required for HTTPS configuration. The UI does not put auth credentials in localStorage or sessionStorage. CSRF tokens are validated for authenticated mutations; cross-origin mutations are rejected. Login additionally requires a custom header. There is no default account or bootstrap password.

The CLI provisions administrators explicitly. The current role model has admin and reviewer: administrators govern identities/policies/lifecycle, and both may review approval requests and read the governance ledger. Account lifecycle APIs, SSO/MFA and fine-grained human roles remain future work. Login throttling is a local per-username/IP foundation, not distributed anti-abuse infrastructure.

## Authorization and execution

Every gateway call requires an agent credential. Current authorization checks agent state, credential revocation, connector enabled state, global stop, required scopes and explicit tool policy. Default is DENY. Explicit ALLOW cannot override missing scope or a tool's mandatory human confirmation. Unknown tools cannot dispatch. Human sessions cannot execute agent tools. Agents cannot use human administration endpoints.

Execution is disabled by configuration and the default catalog contains no external adapters. Input properties and JSON complexity are bounded; registered tool schemas validate inputs and outputs. Adapter registration is trusted code, not user-selected modules or commands. Remote schema references, unrestricted URLs, arbitrary code and agent-supplied credentials are outside the adapter contract.

Adapter results are marked untrusted data. This foundation does not send them to an LLM or treat them as instructions. Future transports must preserve this boundary and protect against prompt injection, malicious responses and cross-application privilege escalation.

## Approval enforcement

Approval binds agent identity, credential, connector, tool, canonical validated parameters and idempotency key through a keyed digest. Binding columns cannot be changed through ordinary SQL updates. No raw parameter values are persisted. Decisions use an atomic pending-state transition, and consumption requires an approved, unexpired matching grant and an enabled approving human.

The client must resubmit the same request after approval. Current authorization is checked before claim and again before dispatch. ASK → ALLOW relaxation still validates an offered grant; DENY tightening blocks it; ALLOW → ASK between claim and dispatch blocks admission without a grant. Approval is consumed once. A decision is permission for this exact request, not a permanent scope grant.

Resource-specific safe previews and requester delegation are not yet implemented; agents cannot assert human requester IDs. Approval summaries currently expose only parameter count/types. External write adapters must add meaningful safe resource summaries before product use.

## Idempotency and uncertainty

Keys are scoped to the agent identity; request collisions with different input fail. A durable prepared record exists before dispatch. A crash or uncertain adapter failure prevents automatic retries; it may require manual reconciliation. Completed replay revalidates authorization and returns a receipt, not a persisted raw business result. Exactly-once external side effects are not claimed.

## Audit and credential handling

Audit records agent/credential or human identity, operation, policy decision, approval/execution reference, timestamp, keyed input digest and correlation ID. It stores shape-only summaries and no request values, passwords, bearer tokens, raw results or exception messages. Unknown tool names are omitted to avoid logging arbitrary caller-controlled identifiers. Audit remains available when agents/credentials are disabled.

SQLite triggers prevent normal updates/deletions of ledger entries. An administrator with filesystem/database access can alter the database; external tamper evidence is planned. Failed bearer authentication is rejected at the transport dependency; a full rejected-auth event ledger is future work.

The owned data directory is mode 0700; key/database are mode 0600. SQLite itself is not encrypted. Runtime data and the key are ignored by Git. The credential key must be backed up with the database, and cannot safely be replaced without rotating credentials and sessions. Outbound connector secrets are not stored because no external adapters ship. A dedicated vault and rotation design are required before those integrations.

## Emergency shutdown

Pause/disable agents, revoke credentials, disable registered connectors or enable global stop to block subsequent admissions. The synchronous foundation serializes control mutations with bounded dispatch under a database write transaction. A control request may wait for current dispatch to finish. These controls cannot cancel already dispatched external operations or undo completed work. Long-running cancellation, leases and provider-specific recovery remain unimplemented.

## Deployment boundary

CLI runtime binds to loopback, debug is off, proxy headers are not implicitly trusted, origins/hosts are constrained, CORS is absent and API docs are not exposed over HTTP. Responses use restrictive CSP, no inline scripts, frame denial, no-store and no-referrer. Production deployments, reverse proxy hardening, request timeouts, secret vaults, distributed rate limits, supply-chain review and security testing require further work. Password KDF cost is preserved from ZNode and needs production sizing/hardening review.
