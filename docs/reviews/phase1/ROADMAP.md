# Roadmap

## Successfully extracted

- Independent ZNode password validation, scrypt hashing and verification.
- Immutable tool-definition model and safe DOM primitives.
- Bearer generation and HMAC hashing functions, with an Agent Control namespace and independent key.

## Adapted and validated

- DENY / ASK / ALLOW policy semantics with a stricter deny fallback and scope-first evaluation.
- Canonical request fingerprinting, now using strict JSON and a keyed digest.
- SQLite foreign-key, WAL, timeout and deterministic connection management foundations.
- Separate agent registry and credential storage, finite human sessions, authentication boundaries and CSRF.
- Request-bound approval queue/decisions, immutable grants, atomic consumption and current-policy revalidation.
- Durable idempotency claims and restart-safe receipts, no blind retries after uncertain dispatch.
- Server-side pause/disable, credential revoke, connector disable and global stop.
- Minimized audit ledger and real-data administration UI.

## Still requiring implementation before external execution

- Confirm the exact source commit running in production; this extraction uses pinned current refactor source.
- Trusted external adapters, endpoint allowlists, egress controls and real provider credential storage.
- Safe resource-specific approval previews; current summaries show parameter shape and omit all values.
- Connector authentication, scoped secret vault, encryption at rest and rotation of connector secrets.
- Agent credential expiry, managed rotation and administrator account lifecycle APIs.
- Worker leases/fencing, bounded outbound I/O, durable dispatch outcomes, reconciliation and cancellation.
- Resource-level policies, delegated human requester authority, tenant boundaries and environmental constraints.
- Idempotency reconciliation and safe retry semantics for individual providers.
- Audit retention/export, externally tamper-evident delivery and backups/restore verification.
- Production load, dependency/advisory audit, operational security review and penetration testing.

## Future product work

GitHub, Google Workspace and Microsoft 365 adapters; internal HTTP/database connectors with least-privilege scopes; MCP transport and OAuth interoperability; SSO/MFA; notification delivery; agent health monitoring; full integration administration; dashboards and resource-level policy editors.

No external integration, MCP transport, cancellation or production security-readiness claim is part of v0.1.0.
