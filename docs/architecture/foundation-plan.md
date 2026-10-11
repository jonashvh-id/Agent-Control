# Foundation extraction plan

Source: `jonashvh-id/zetako-node`, refactor commit `9bd840ba6b5fc285f178a0383709c653dd1e5aa8`. Local checkouts were inventoried and hashed before work. The current main branch is older; refactor feeds the pilot release workflow. Live server provenance is unverified.

Reuse the independent password functions, agent tool definition, bearer-token generation/hash functions, request canonicalization and safe DOM helpers. Adapt policy decisions with a strict deny fallback and scope checks. Extract the SQLite connection settings without ZNode instrumentation. Replace the coupled storage and orchestration with focused standalone repositories and services. Preserve request fingerprinting, current-policy replay checks and lifecycle enforcement; strengthen approval consumption with transactional compare-and-set.

Modules: identity owns human sessions; agents owns registry and credentials; policies owns deterministic decisions; approvals owns immutable grants; gateway owns the control sequence; connectors owns the trusted catalog and enabled state; execution owns durable idempotency claims; audit owns minimized events; lifecycle owns emergency state.

Dependency flow: API → composed services → repositories / pure security and policy helpers → SQLite. Gateway composes modules and calls the adapter interface. External adapter implementations belong in `connectors/`, never API routes. No production adapter is registered initially.

Schema: humans, sessions, login attempts, agents, agent credentials, policies, connector state, approvals, execution claims, audit events, emergency state, schema version. All runtime tables live in a new owned data directory. No ZNode migrations or databases are used.

Frontend: dependency-free ES modules served by FastAPI at the same origin. Server sessions, CSRF tokens, live data, empty states, agent registration/lifecycle, policies and approval decisions. External integrations and MCP transport are explicitly planned. No chatbot or collaboration UI.

Validation: isolated integration tests for auth boundaries, strict deny, revoked credentials, fingerprint tampering, policy tightening/relaxation, one-time approvals, idempotency collisions/restart, emergency stop and redaction. Start an installed backend from a different working directory with new runtime data. Compare every source hash, Git status, HEAD and ref against the recorded ZNode baseline.
