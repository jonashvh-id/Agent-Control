from types import SimpleNamespace

import pytest
from fastapi.testclient import TestClient

from backend.app.core.config.settings import Settings
from backend.app.core.security.passwords import hash_password
from backend.app.main import create_app
from backend.app.modules.connectors.models import AgentToolDefinition
from backend.app.modules.connectors.registry import ConnectorRegistry


class TestOnlyAdapter:
    __test__ = False

    def __init__(self):
        self.calls = []
        self.failure = False

    def execute(self, tool, parameters, *, idempotency_key):
        self.calls.append((tool, parameters.copy(), idempotency_key))
        if self.failure:
            raise RuntimeError("token=must-never-be-exposed")
        return {"recorded": True}


@pytest.fixture
def harness(tmp_path):
    adapter = TestOnlyAdapter()
    registry = ConnectorRegistry()
    definition = AgentToolDefinition(
        "records.create",
        "Test-only adapter",
        {
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "required": ["name"],
            "additionalProperties": False,
        },
        {
            "type": "object",
            "properties": {"recorded": {"type": "boolean"}},
            "required": ["recorded"],
            "additionalProperties": False,
        },
        ("records:write",),
        "high",
        False,
    )
    registry.register("test", definition, adapter)
    settings = Settings(tmp_path / "runtime", execution_enabled=True)
    app = create_app(settings, registry)
    with TestClient(app, base_url="http://127.0.0.1:8010") as client:
        services = app.state.services
        with services.database.connect(write=True) as db:
            services.identity.repo.create(db, "admin", hash_password("Foundation9!Secure"))
            services.identity.repo.create(db, "reviewer", hash_password("Foundation9!Review"), "reviewer")
        login = client.post(
            "/api/v1/auth/login",
            json={"username": "admin", "password": "Foundation9!Secure"},
            headers={"x-control-request": "1"},
        )
        assert login.status_code == 200, login.text
        actor = login.json()
        client.headers["x-csrf-token"] = actor["csrf"]
        agent = client.post("/api/v1/agents", json={"name": "Test agent", "scopes": ["records:write"]}).json()
        credential = client.post(
            f"/api/v1/agents/{agent['id']}/credentials", json={"scopes": ["records:write"]}
        ).json()
        services.lifecycle.set_connector(actor, "test", True)
        h = SimpleNamespace(
            client=client,
            services=services,
            actor=actor,
            agent=agent,
            credential=credential,
            adapter=adapter,
            app=app,
            settings=settings,
            registry=registry,
        )
        h.request = {
            "connector": "test",
            "tool": "records.create",
            "parameters": {"name": "Sensitive customer name"},
            "idempotency_key": "request-key-001",
        }
        h.headers = {"authorization": f"Bearer {credential['bearer_token']}"}
        h.policy = lambda mode: client.put(
            "/api/v1/policies",
            json={"agent_id": agent["id"], "connector": "test", "tool": "records.create", "mode": mode},
        )
        h.execute = lambda **overrides: client.post(
            "/api/v1/gateway/execute", json={**h.request, **overrides}, headers=h.headers
        )
        yield h
