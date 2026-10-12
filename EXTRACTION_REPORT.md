> Historical Python extraction report. The primary backend is now Rust; destinations below are preserved under `reference/python/` and in the pinned Git history. See RUST_MIGRATION_REPORT.md for the current migration.

# Extraction report

## Measured scope of reuse (phase 1)

Reuse was limited and selective, not a wholesale extraction of ZNode's services. Two files were copied as complete standalone helpers (`passwords.py` and the tool model), two were isolated from existing primitives (token functions and DOM helpers). Those four destination files contain **169 physical lines / 136 nonblank, non-comment lines** after formatting. Four other files adapt the policy, canonical hashing, safe summaries and SQLite connection foundations: **106 physical lines in the original phase-1 snapshot**. These figures describe destination files; they are not a claim that every line is unchanged source.

Identity sessions, agent registry/credential repositories, approval persistence, durable execution state, lifecycle orchestration, audit storage, API routing and most UI code were **new standalone implementations** guided by source patterns. The phase-1 backend contained 906 executable Python statements according to the coverage run. The copied-helper quantities include one JavaScript file and therefore must not be compared directly with that Python statement count as a percentage.

The previous broad phrase “extracted governance foundation” should be read with that limitation. ZNode's complete governance services were too coupled to excluded user/messaging/business subsystems to copy intact. Their behavior was replaced, not inherited with all original tests.

The 33 original tests were rerun before phase-2 changes with branch coverage: 88% overall; gateway service 96%, approval repository 96%, policy engine 80%, identity service 100%, agent service 90%. Coverage is diagnostic, not an independent audit or a security-readiness claim. Full per-file line/branch details are in `docs/reviews/phase1-coverage.json` and the findings in `docs/reviews/FOUNDATION_REVIEW.md`.

The extracted-component manifest remains historical phase-1 provenance. Later changes are listed in the phase-2 report; its destination hashes are not expected to match files modified in later phases.

## Source selection and preservation

Repository: `jonashvh-id/zetako-node`. Inspected remote branches and nine local checkouts. Local primary checkout was `test-refactor` at `69bbfa55e8b1ceb66cd7a76c622989b4bd4ba1d2`, with a pre-existing deleted document and two untracked files. These were left in place. Exact HEADs, refs, status and SHA-256 hashes of all tracked/non-ignored untracked source files are in `docs/migration/znode-before.json`.

Remote main was `234aa1ac15215fc5718a07ce7ebdfa60561c25ad` (September 7). Remote refactor was `9bd840ba6b5fc285f178a0383709c653dd1e5aa8` (October 8), and the pilot release workflow names refactor. Extraction uses that pinned refactor commit, fetched as allowlisted source blobs into a separate temporary inspection directory. No checkout was fetched, switched, branched, edited, deployed or executed. Live production server commit was not available through GitHub deployment/release metadata; do not treat branch selection as proof of production provenance.

No .env file, token, runtime database, customer data, deployment secret or customer-specific config was downloaded or copied. Local source preservation uses hashes, not copies of runtime data. No symlinks connect the projects.

## Code selectively copied and adapted

| Source | Destination | Treatment |
|---|---|---|
| `app/services/passwords.py` | `backend/app/core/security/passwords.py` | Full independent helper; formatting/import ordering only; scrypt logic retained |
| `app/models/agent_tools.py` | `backend/app/modules/connectors/models.py` | Independent frozen tool-definition model; formatting only |
| `app/services/agent_integration_service.py` | `core/security/tokens.py` | Extracted generate_token/hash_token functions; changed namespace to ac_agent; no UserService/audit dependencies |
| `app/services/agent_capability_policy_service.py` | `modules/policies/engine.py` | Adapted effective_mode/effective_tool; scope check before policy; unconditional deny fallback; removed server.prune special case in favor of tool metadata |
| `app/services/agent_action_audit_service.py` | `core/security/fingerprint.py`, `modules/audit/redaction.py` | Adapted canonical JSON binding and safe summaries; strict finite JSON, keyed HMAC; removed arbitrary value/key previews |
| `zetako/sqlite.py` | `core/database/connection.py` | Adapted FK/WAL/busy-timeout setup and deterministic connection close; removed contextvar caches and ZNode telemetry |
| `app/frontend/static/js/user_dom.js` | `frontend/src/components/dom.js` | Copied safe DOM primitives; renamed global to AgentControlDom |

Destinations beginning `core/` or `modules/` are relative to `backend/app`. Function/file provenance and destination digests are recorded in `docs/migration/extracted-components.json`. Copies live exclusively in the new project.

## Adaptation rather than wholesale copying

Agent integration records/stores were tied to shared ZNode users, settings tables and messaging scope defaults. The new registry and credential repositories retain independent identity/provider/scope/lifecycle concepts, with distinct humans and agents. They do not copy the whole user service. Human sessions use the reused password helper and bearer hashing primitive, with new focused persistence and transport dependencies.

The source policy engine allowed implicit execution when scope matched and no policies existed. Agent Control deliberately tightens this to DENY for every unconfigured tool. Explicit modes cannot bypass scopes.

Source approval and gateway services depend on conversation origin messages, human user roles, chat cards, runtime dispatch and ZNode's tool catalog. Their request-hash binding, expiry, replay revalidation and lifecycle patterns were adapted into new standalone services. No conversation requester delegation is emulated. Approval binding and consumption are strengthened with immutable columns and conditional state changes. A durable execution claim blocks uncertain retries, while raw result caching is excluded.

The source action audit service has tool-specific summaries for ZNode business payloads. The new ledger records minimal shape-only summaries and operation metadata, not messages, file data, credentials or arbitrary text previews. Deleted/disabled agents do not erase audit history.

The source admin HTML/CSS includes ZNode principal roles, integrations and collaboration navigation. Only generic DOM helpers were reusable without branding/dependency coupling. The dashboard is an independent responsive UI backed by the new API; no events or statistics are seeded.

## Dependencies retained and removed

Retained FastAPI/Uvicorn, Python standard-library sqlite3, scrypt/HMAC primitives. Added JSON Schema validation for trusted adapter contracts. Test tooling uses pytest, Ruff and Starlette's current httpx2 client. The exact validated environment is locked in `requirements.lock`.

Removed ZNode package imports/config globals, UserService, MessagingService, workspace ACL services, calendars/meetings, AgentIntelligenceService, managed providers, media services, toolkit handlers, background connector processes, PDF dependencies, push notifications, WebAuthn/PyQt, mail configuration and runtime deployment scripts. UI needs no provider SDK, npm dependency or frontend build service.

## Tests migrated by behavior

Original security-sensitive tests depend on ZNode's shared `app_harness`, storage handlers, messaging and provider bridge. They were inspected, not run or copied wholesale. Standalone replacements cover source foundation policy controls, idempotency restart/replay, stale approved requests after ASK → ALLOW/DENY changes, scope restrictions, revocation, human/agent separation and immutable confirmation. Extra tests cover concurrent claims, expiry, changed parameters/keys/credentials, uncertain adapter failure, current admission checks and audit minimization.

No claim is made that the entire ZNode test suite passed here. Test-only adapters implement actual deterministic fixture behavior and are never registered by the normal application or included in the wheel.

## Remaining debt and blockers

Live production provenance remains unconfirmed. Full principal/auth/approval/audit services cannot be copied unchanged without dragging excluded subsystems into this repository. Their standalone replacements need further operational and security review. Scrypt cost is preserved; production hardening is pending. Runtime SQLite is not encrypted, ledger is not externally tamper-evident, resource previews are shape-only, credential expiry/secret vaults/SSO are absent, and bounded synchronous dispatch is not ready for long-running external integrations.

No residual runtime coupling to ZNode exists. The optional preservation script and migration documentation contain source paths only for verification/provenance, and are not imported by the backend.

## Source files examined

- `pyproject.toml` (`5461ddb423a4`)
- `app/main.py` (`20028a023e1c`)
- `app/models/agent_actions.py` (`fb1920fa9102`)
- `app/models/agent_integrations.py` (`230ae564e302`)
- `app/models/agent_confirmations.py` (`270776d628e9`)
- `app/services/agent_capability_policy_service.py` (`23c22e1a0727`)
- `app/services/agent_principal_service.py` (`c5f42a09474c`)
- `app/services/agent_integration_service.py` (`77e11e961d10`)
- `app/services/agent_integration_store.py` (`a705e7bbdabf`)
- `app/services/agent_confirmation_service.py` (`97119c5c8c8a`)
- `app/services/agent_action_audit_service.py` (`5917832b9af7`)
- `app/services/agent_execution_gateway.py` (`c3bafa77c2e3`)
- `app/services/agent_execution_application.py` (`d1fa31378a0c`)
- `app/services/agent_tool_runtime.py` (`f7da09b8d7af`)
- `app/services/agent_tool_dispatcher.py` (`eea15f5a94ac`)
- `app/services/auth_service.py` (`f8089b404208`)
- `app/services/auth_store.py` (`2adf4625ec0c`)
- `app/services/security_service.py` (`d05da5859084`)
- `app/services/audit_store.py` (`18c68f2edb29`)
- `app/routes/admin/agents.py` (`3b44e92b9f72`)
- `app/frontend/templates/pages/agents.html` (`fabe4492e017`)
- `app/frontend/templates/pages/agent_detail.html` (`6bd959db7b28`)
- `app/frontend/static/css/dashboard/agents.css` (`a8e773e68c3f`)
- `tests/test_agent_execution_replay.py` (`aaffc262ca5a`)
- `tests/test_agent_policy_relaxation_replay.py` (`608cab9f2edf`)
- `tests/test_agent_approval_resume.py` (`a11474f014a0`)
- `tests/test_agent_center.py` (`41d2708a85d7`)
- `tests/test_agent_foundation_e2e.py` (`b41da643920c`)
- `.github/workflows/agent-foundation.yml` (`a53ce8b7bcb8`)
- `app/services/passwords.py` (`344798f37285`)
- `zetako/sqlite.py` (`a54b8a5ee8d5`)
- `app/models/agent_tools.py` (`01b2e99db280`)
- `app/frontend/static/js/user_dom.js` (`6ab05a8219d8`)
