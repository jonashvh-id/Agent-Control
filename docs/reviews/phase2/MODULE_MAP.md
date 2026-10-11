# Module map — GitHub/MCP pilot

| Module | Responsibility |
|---|---|
| `backend/app/main.py` | App factory, parent/SDK lifespan, HTTP router/static composition |
| `backend/app/bootstrap.py` | Explicit dependency composition and trusted connector registration |
| `backend/app/core/config/settings.py` | Operator environment, origin and activation flags |
| `core/database/connection.py`, `migrations.py` | Owned SQLite runtime and additive standalone migrations |
| `core/security/passwords.py`, `tokens.py` | Retained password/token primitives |
| `core/security/fingerprint.py`, `sealed.py` | Exact request binding and encrypted approval previews |
| `modules/identity` | Human sessions, roles, CSRF, login throttling |
| `modules/agents` | Separate agent identities, intersected scopes, credential issuance/revocation |
| `modules/policies` | Pure deterministic DENY/ASK/ALLOW decision and audited policy mutations |
| `modules/approvals` | Immutable request grants, encrypted previews, decisions, expiry, one-time consumption |
| `modules/gateway/service.py` | Small facade shared by REST and MCP |
| `modules/gateway/authorization.py` | One current identity/scope/policy/lifecycle check |
| `modules/gateway/admission.py` | Validation, immutable approval and durable idempotency claim |
| `modules/gateway/dispatch.py` | Final dispatch admission, bounded I/O, validated result and outcome |
| `modules/gateway/journal.py` | Minimized, correlated governance events |
| `modules/execution` | Durable claim, dispatch marker and minimized result receipt |
| `modules/lifecycle` | Agent/connector state and global emergency admission stop |
| `modules/audit` | Append-only application ledger; no raw titles/bodies/secrets |
| `modules/connectors/registry.py` | Trusted adapter interface/catalog; no dynamic imports or remote schemas |
| `modules/connectors/github_config.py` | Finite repository allowlist and credential-source configuration |
| `modules/mcp/authentication.py` | Revalidate bearer on every MCP HTTP request; no cookie/session shortcut |
| `modules/mcp/contracts.py` | Governed-tool MCP schemas; rejects transport authority overrides |
| `modules/mcp/server.py` | Official SDK Streamable HTTP handlers; delegates to shared gateway |
| `connectors/github/adapter.py` | Real fixed-origin GitHub REST issue read/create; no deletion or retries |
| `connectors/github/credentials.py` | Read one private token file or local gh keychain reference at dispatch |
| `connectors/github/tools.py`, `registration.py` | Two tool contracts and explicit adapter registration |
| `api/v1` | Thin human management and agent gateway HTTP transports |
| `frontend/src/features/approvals.js` | Display exact preview as inert text and record authenticated human decision |
| Other frontend files | Existing administration shell, live data and configuration status |
| `scripts/mcp_issue_client.py` | Independent external MCP client with only a platform credential |
| `scripts/github_pilot.py` | Isolated live probe, explicit local operator approval and at most one issue write |
| Tests | Original foundation regressions, provider boundaries and SDK/network end-to-end cases |

`core`, `modules` and `api` paths are relative to `backend/app`. External adapter code is in top-level connectors; operator configuration and administration remain backend-owned. No module imports ZNode or an old source tree. The phase-1 map is archived under `docs/reviews/phase1`.
