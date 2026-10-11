# Agent Control engineering instructions

This repository is a standalone governance foundation with a restricted GitHub/MCP pilot, not an enterprise security certification.

- ZNode is read-only. Never edit its source, Git state, deployments, configuration or data. Never run its application or tests against its runtime. Copy only reviewed source; never copy secrets, databases or customer data.
- One module has one responsibility. Separate API routing, services, persistence, policy decisions and connector adapters. Prefer files under 300 lines; review responsibilities above 400 lines. Do not create giant agents/security/gateway/utils modules.
- Composition root is `backend/app/bootstrap.py`. Dependencies flow inward; modules do not import routes or the application object. Pass transactions explicitly when multiple changes must be atomic.
- Humans and agents have separate authentication paths. Agents never receive human roles. Enforce authorization server-side and deny tools without an explicit policy.
- Approval binds agent, connector, tool, validated parameters and idempotency key. Revalidate current authorization before consumption. Never reuse a consumed approval or retry an uncertain write automatically.
- No arbitrary code, shell commands, user-supplied URLs or dynamic adapter imports in execution. Register trusted adapters in code. The GitHub pilot adapter is explicitly configured and disabled by default; ZNode is always excluded.
- No credentials, request values or raw connector results in audit. Preserve records when credentials or agents are disabled.
- UI uses live API data and explicit empty/future states. Never fabricate activity, integrations or metrics.
- Run `.venv/bin/pytest`, frontend checks and the standalone smoke test after security-sensitive changes. Keep tests isolated with fresh temporary databases and test-only adapters.
- Document extraction provenance and security limitations. Do not deploy publicly or change production infrastructure as part of foundation work.
