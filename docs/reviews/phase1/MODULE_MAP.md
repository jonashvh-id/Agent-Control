# Module map

| Path | Responsibility / interface |
|---|---|
| `backend/app/main.py` | App factory, lifespan, route assembly, safe errors and static UI |
| `backend/app/bootstrap.py` | Composition of explicit service dependencies |
| `backend/app/cli.py` | Loopback runtime and prompted administrator bootstrap |
| `core/config/settings.py` | Process configuration and origin validation |
| `core/database/connection.py` | Owned runtime initialization and transaction lifecycle |
| `core/database/schema.sql` | Initial standalone schema, constraints and immutability triggers |
| `core/security/passwords.py` | Extracted scrypt hashing and password checks |
| `core/security/tokens.py` | Extracted bearer generation and HMAC lookup hashes |
| `core/security/fingerprint.py` | Strict canonical JSON and keyed request binding |
| `core/security/middleware.py` | Browser origin boundary, body size and security headers |
| `core/errors/domain.py` | Sanitized domain error codes and HTTP status mapping |
| `modules/identity` | Human authentication, server-side sessions, CSRF and local login attempts |
| `modules/agents` | Agent registry, credential scopes and revocation; no human role inheritance |
| `modules/policies` | Pure DENY / ASK / ALLOW engine and explicit policy storage |
| `modules/approvals` | Immutable grants, finite expiry, reviewer decisions and atomic consumption |
| `modules/audit` | UTC event timestamps, minimized summaries and append-only ledger reads |
| `modules/lifecycle` | Agent status, connector status and global emergency state |
| `modules/connectors` | Tool definitions, trusted adapter protocol and registration catalog |
| `modules/execution` | Durable idempotency claims and executed/failed state |
| `modules/gateway` | Strict proposals, centralized authorization, discovery and governed dispatch |
| `api/v1/dependencies.py` | Shared human/reviewer/admin and agent transport authentication |
| `api/v1/schemas.py` | Strict allowlisted administration input models |
| `api/v1/*` | Thin identity, registry, governance and gateway routes |
| `connectors/README.md` | External adapter implementation rules and current unimplemented status |
| `frontend/src/services/api.js` | Same-origin API client with memory-only CSRF |
| `frontend/src/components` | Safe DOM primitives, dialogs and independent navigation shell |
| `frontend/src/features` | Agent management, policy editor and approval review |
| `frontend/src/pages` | Login, overview, audit, integrations, security and settings |
| `frontend/src/styles` | Responsive enterprise UI and layout |
| `setup.py` | Bundle frontend sources into distribution wheels |
| `scripts` | Standalone smoke check and read-only source verification |
| `backend/tests`, `tests/integration`, `frontend/tests` | Isolated regressions and UI checks |

Paths under `core`, `modules` and `api` in the table are relative to `backend/app`. Repositories never import service modules or routes. External adapters never call human administration routes. Gateway security is shared across transport extensions.
