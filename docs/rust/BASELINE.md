# Rust migration baseline

Reference: `b946766e705856131b59f386b5ef657310f7dfdf`. Dedicated branch: `migration/rust-backend`. Initial worktree clean; Python suite independently rerun: 65 passed in 6.18 seconds. Phase 2 report read. Python source remains intact during implementation; Git history preserves the reference indefinitely.

Contract inventory: `/health`, `/`, `/ui/*`; `/api/v1/auth/{login,me,logout}`; agents list/create/details/status, credential issue/revoke; policies list/set; approvals list/decision; audit; integrations list/status; settings; emergency-stop; gateway tools/execute; MCP Streamable HTTP `/mcp/`. Humans use HttpOnly sessions and CSRF, agents use scoped bearer credentials. Current enabled identity, credential, policy, scopes and lifecycle govern discovery and execution.

Persistence inventory: humans, sessions, login_attempts, agents, credentials, policies, connector_state, emergency_state, approvals, executions, audit_events. Python schema version 1 plus additive migration 2. Approval bindings/previews immutable; audit append-only. Rust uses a separately marked new runtime and new migrations; it never automatically opens historical/customer databases.

Parity gates: every essential endpoint and existing UI flow, independent MCP SDK transport, default-deny/scope intersection/mandatory issue confirmation, exact encrypted one-time previews, expiry/current approver/current policy, all revocations, durable idempotency under concurrency/restart, uncertain-write protection, bounded fixed-origin provider I/O, sanitized errors/audit, independent executable startup. Fixtures remain isolated; no additional live provider writes.

Rust adds explicit execution states for dispatch and unknown outcomes. Python classified failed or unfinished execution retries as uncertain; Rust must retain that refusal while recording the outcome precisely. A revocation prevents new admission, not cancellation of admitted external I/O.

Source protection: historical ZNode verifier compares nine checkouts and 11,858 file hashes. Dovehouse files and Git state captured read-only before migration in the private ignored `var/rust-migration/dovehouse-before.json`. Neither project is part of the runtime or test harness.
