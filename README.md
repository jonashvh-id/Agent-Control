# Agent Control

Standalone governance platform foundation with a restricted **GitHub/MCP pilot**. It governs only requests routed through its gateway. This release is not an enterprise production security certification.

Working: human sessions, separate agent identities and scoped credentials, strict DENY/ASK/ALLOW, encrypted exact human approval previews, audit/receipts/revocation, the existing administration shell, official SDK Streamable HTTP MCP and a real GitHub issue adapter. No chatbot/LLM or collaboration subsystem is included.

## Run locally

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements.lock
.venv/bin/python -m pip install --no-deps -e .
.venv/bin/agent-control create-admin --username admin
.venv/bin/agent-control serve
```

Python 3.11+; npm is needed only for frontend checks. Open http://127.0.0.1:8010. No default password/account. The CLI binds to loopback. Choose an empty owned AGENT_CONTROL_DATA_DIR; never point it at ZNode. Environment examples are documentation, not automatically loaded.

## Configure the GitHub pilot

Use a dedicated fine-grained GitHub token scoped to one test repository, with Issues read/write and required metadata access. Save it in an operator-owned, nonsymlink mode-0600 file outside source control; never place it in agent arguments or UI configuration.

```sh
export AGENT_CONTROL_GITHUB_REPOSITORIES=jonashvh-id/Agent-Control
export AGENT_CONTROL_GITHUB_CREDENTIAL_SOURCE=file
export AGENT_CONTROL_GITHUB_TOKEN_FILE=/absolute/private/path/github.token
export AGENT_CONTROL_EXECUTION_ENABLED=true
.venv/bin/agent-control serve
```

The connector still initializes disabled. Enable it through the administrator integration API and set explicit policies for the chosen agent: issues.list ALLOW, issues.create ASK. Scope the agent and its credential to issues:read/issues:write as needed. No tool executes merely because it was registered, configured or proposed by a model. Native GitHub permissions remain authoritative.

For a minimal local demonstration only, gh-demo may replace file credentials. This references the existing gh keychain access in memory and is restricted to jonashvh-id/Agent-Control and loopback. Native token rights may be broader than the platform's restriction; use a dedicated credential for an operational deployment.

```sh
.venv/bin/python scripts/github_pilot.py
```

The script creates an isolated development runtime/operator and starts an independent child MCP client. It performs one read, prepares one exact ASK proposal, and waits for the local human operator to type `approve <approval_id>`. EOF or any other answer denies without a write. It can issue at most one create request and then checks revocation. Runtime credentials are private/ignored; GitHub credentials are not copied or printed. Use --read-proof to reuse an existing successful read without another provider read.

## Validate

```sh
.venv/bin/pytest -q
.venv/bin/ruff check backend connectors scripts tests setup.py
(cd frontend && npm run check)
.venv/bin/python scripts/smoke_standalone.py
.venv/bin/python scripts/verify_znode_unchanged.py
```

The optional last command reads historical source/Git hashes only; ZNode is not a runtime dependency. Tests use isolated native provider responses and a real SDK/network transport; live GitHub evidence is reported separately. Wheels bundle UI/SQL and the trusted adapter without tests, runtime databases or credentials.

See [phase 2 results and live evidence](docs/reviews/PHASE2_REPORT.md), [extraction details](EXTRACTION_REPORT.md), [foundation review](docs/reviews/FOUNDATION_REVIEW.md), [module map](MODULE_MAP.md), [architecture](ARCHITECTURE.md), [security boundaries](SECURITY_BOUNDARIES.md), [MCP API](docs/api/MCP_GITHUB.md) and [roadmap](ROADMAP.md).

Remaining: OAuth MCP, SSO/MFA, managed vault/rotation, fine-grained human/tenant controls, cancellation/reconciliation, external tamper-evident audit and production/load/security certification. No public deployment or production change is included.
