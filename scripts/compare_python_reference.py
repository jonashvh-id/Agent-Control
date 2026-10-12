"""Optional migration tooling: compare pinned Python behavior with an isolated Rust fixture server."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import io

BASELINE = "b946766e705856131b59f386b5ef657310f7dfdf"


def scenario(client, prefix=""):
    transcript = []
    csrf = ""
    def call(name, method, path, body=None, token=None, expected=200, select=()):
        headers = {"x-control-request": "1", "x-csrf-token": csrf}
        if token:
            headers["authorization"] = "Bearer " + token
        response = client.request(method, prefix + path, json=body, headers=headers)
        assert response.status_code == expected, (name, response.status_code, response.text)
        value = response.json() if response.status_code != 204 else None
        transcript.append({"case": name, "status": response.status_code, "values": {key: value[key] for key in select}})
        return value
    login = call("login", "POST", "/api/v1/auth/login", {"username": "admin", "password": "Foundation9!Secure"}, select=("role",))
    csrf = login["csrf"]
    call("me", "GET", "/api/v1/auth/me", select=("username", "role"))
    agent = call("create", "POST", "/api/v1/agents", {"name": "Parity agent", "scopes": ["issues:read", "issues:write"]}, expected=201, select=("name", "status", "scopes"))
    aid = agent["id"]
    credential = call("issue", "POST", f"/api/v1/agents/{aid}/credentials", {"scopes": ["issues:read", "issues:write"]}, expected=201, select=("display_once",))
    token = credential["bearer_token"]
    request = {"connector": "github", "tool": "issues.create", "parameters": {"repository": "demo/repo", "title": "Exact parity title", "body": "Exact parity body"}, "idempotency_key": "parity-create-001"}
    call("default deny", "POST", "/api/v1/gateway/execute", request, token, 403, ("error",))
    call("enable", "PATCH", "/api/v1/integrations/github", {"enabled": True}, select=("enabled",))
    policy = {"agent_id": aid, "connector": "github", "tool": "issues.list", "mode": "allow"}
    call("read policy", "PUT", "/api/v1/policies", policy, select=("mode", "tool_registered"))
    read = {**request, "tool": "issues.list", "parameters": {"repository": "demo/repo"}, "idempotency_key": "parity-read-001"}
    call("read", "POST", "/api/v1/gateway/execute", read, token, select=("status", "result_trust"))
    call("creation allow coerces ask", "PUT", "/api/v1/policies", {**policy, "tool": "issues.create"}, select=("mode",))
    pending = call("ask", "POST", "/api/v1/gateway/execute", request, token, 202, ("status",))
    approvals = call("list approvals", "GET", "/api/v1/approvals")
    preview = next(a for a in approvals if a["id"] == pending["approval_id"])
    assert preview["input_preview"] == {**request["parameters"], "operation": "issues.create"}
    assert len(preview["input_hash"]) == 64
    approved = {**request, "approval_id": pending["approval_id"]}
    # Separate human API session decides the immutable proposal.
    call("approve", "POST", f"/api/v1/approvals/{pending['approval_id']}/decision", {"decision": "approved"}, select=("status",))
    call("changed parameters", "POST", "/api/v1/gateway/execute", {**approved, "parameters": {**request["parameters"], "title": "Changed"}}, token, 403, ("error",))
    call("execute", "POST", "/api/v1/gateway/execute", approved, token, select=("status", "receipt", "result_trust"))
    call("replay", "POST", "/api/v1/gateway/execute", approved, token, select=("status", "result_available", "receipt"))
    call("idempotency conflict", "POST", "/api/v1/gateway/execute", {**approved, "parameters": {**request["parameters"], "body": "Changed"}}, token, 409, ("error",))
    call("credential escalation", "POST", f"/api/v1/agents/{aid}/credentials", {"scopes": ["repos:delete"]}, expected=403, select=("error",))
    call("pause", "PATCH", f"/api/v1/agents/{aid}/status", {"status": "paused"}, select=("status",))
    call("inactive", "GET", "/api/v1/gateway/tools", token=token, expected=403, select=("error",))
    call("activate", "PATCH", f"/api/v1/agents/{aid}/status", {"status": "active"}, select=("status",))
    call("global stop", "PUT", "/api/v1/security/emergency-stop", {"stopped": True}, select=("stopped",))
    call("stopped dispatch", "POST", "/api/v1/gateway/execute", approved, token, 403, ("error",))
    call("resume", "PUT", "/api/v1/security/emergency-stop", {"stopped": False}, select=("stopped",))
    call("revoke", "DELETE", f"/api/v1/agents/{aid}/credentials/{credential['id']}", expected=204)
    call("revoked", "GET", "/api/v1/gateway/tools", token=token, expected=401, select=("error",))
    events = call("audit", "GET", "/api/v1/audit")
    serialized = json.dumps(events)
    assert token not in serialized and "Exact parity title" not in serialized and "Exact parity body" not in serialized
    execution = next(e for e in events if e["action"] == "execution.executed" and e["tool"] == "issues.create")
    assert execution["approval_id"] == pending["approval_id"] and execution["agent_id"] == aid and execution["correlation_id"]
    call("logout", "POST", "/api/v1/auth/logout", expected=204)
    call("logged out", "GET", "/api/v1/auth/me", expected=401, select=("error",))
    return transcript


def main():
    import httpx2
    origin = os.environ["AGENT_CONTROL_RUST_TEST_ORIGIN"]
    archive = subprocess.run(["git", "archive", BASELINE], check=True, capture_output=True).stdout
    with tempfile.TemporaryDirectory(prefix="ac-python-parity-") as tmp:
        root = Path(tmp)
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            tar.extractall(root / "reference", filter="data")
        sys.path.insert(0, str(root / "reference"))
        from backend.app.core.config.settings import Settings
        from backend.app.core.security.passwords import hash_password
        from backend.app.main import create_app
        from backend.app.modules.connectors.registry import ConnectorRegistry
        from backend.app.modules.connectors.github_config import GitHubConfig
        from connectors.github.adapter import GitHubAdapter
        from connectors.github.tools import definitions
        from fastapi.testclient import TestClient
        secret = root / "github.token"
        secret.write_text("fixture-private-token")
        secret.chmod(0o600)
        config = GitHubConfig(("demo/repo",), secret)
        def provider(request):
            payload = {"number": 7, "title": "Read title", "html_url": "https://github.com/demo/repo/issues/7", "state": "open"}
            if request.method == "POST":
                payload["title"] = json.loads(request.content)["title"]
                return httpx2.Response(201, json=payload)
            return httpx2.Response(200, json=[payload])
        registry = ConnectorRegistry()
        adapter = GitHubAdapter(config, transport=httpx2.MockTransport(provider))
        for tool in definitions(config):
            registry.register("github", tool, adapter)
        settings = Settings(root / "runtime", execution_enabled=True, github=config)
        app = create_app(settings, registry)
        with TestClient(app, base_url=settings.origin) as client:
            services = app.state.services
            with services.database.connect(write=True) as db:
                human_id = services.identity.repo.create(db, "admin", hash_password("Foundation9!Secure"))
            services.lifecycle.set_connector({"id": human_id}, "github", True)
            python = scenario(client)
        with httpx2.Client(trust_env=False, timeout=15) as client:
            rust = scenario(client, origin)
        assert python == rust, [(p, r) for p, r in zip(python, rust) if p != r]
        print(json.dumps({"baseline": BASELINE, "equivalent_http_observations": len(rust), "passed": True}))


if __name__ == "__main__":
    main()
