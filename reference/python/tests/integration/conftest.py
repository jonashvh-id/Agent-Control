import socket
import threading
import time
from types import SimpleNamespace

import httpx2
import pytest
import uvicorn

from backend.app.bootstrap import compose
from backend.app.core.config.settings import Settings
from backend.app.core.security.passwords import hash_password
from backend.app.main import create_app
from backend.app.modules.connectors.github_config import GitHubConfig
from backend.app.modules.connectors.registry import ConnectorRegistry
from connectors.github.adapter import GitHubAdapter
from connectors.github.tools import definitions


@pytest.fixture
def github_server(tmp_path):
    provider_calls = []
    provider_status = {"code": 201}
    token_file = tmp_path / "github.token"
    token_file.write_text("test-native-github-token")
    token_file.chmod(0o600)
    config = GitHubConfig(("demo/repo",), token_file)

    def native(request):
        provider_calls.append(request)
        created = {
            "number": 7,
            "title": "Verified issue",
            "html_url": "https://github.com/demo/repo/issues/7",
            "state": "open",
        }
        return (
            httpx2.Response(200, json=[created])
            if request.method == "GET"
            else httpx2.Response(provider_status["code"], json=created)
        )

    adapter = GitHubAdapter(config, transport=httpx2.MockTransport(native))
    registry = ConnectorRegistry()
    for definition in definitions(config):
        registry.register("github", definition, adapter)
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    origin = f"http://127.0.0.1:{port}"
    settings = Settings(tmp_path / "data", origin=origin, execution_enabled=True)
    seed = compose(settings, registry)
    with seed.database.connect(write=True) as db:
        seed.identity.repo.create(db, "operator", hash_password("Fixture9!Operator"))
    app = create_app(settings, registry)
    server = uvicorn.Server(
        uvicorn.Config(app, host="127.0.0.1", port=port, log_level="error", access_log=False)
    )
    thread = threading.Thread(target=server.run, daemon=True)
    thread.start()
    with httpx2.Client(base_url=origin, timeout=5, trust_env=False) as client:
        for _ in range(100):
            try:
                if client.get("/health").status_code == 200:
                    break
            except httpx2.TransportError:
                pass
            time.sleep(0.02)
        else:
            raise RuntimeError("Test server failed to start")
        human = client.post(
            "/api/v1/auth/login",
            headers={"x-control-request": "1"},
            json={"username": "operator", "password": "Fixture9!Operator"},
        ).json()
        client.headers["x-csrf-token"] = human["csrf"]
        agent = client.post(
            "/api/v1/agents",
            json={"name": "External MCP test client", "scopes": ["issues:read", "issues:write"]},
        ).json()
        credential = client.post(
            f"/api/v1/agents/{agent['id']}/credentials", json={"scopes": ["issues:read", "issues:write"]}
        ).json()
        client.patch("/api/v1/integrations/github", json={"enabled": True})

        def policy(tool, mode):
            response = client.put(
                "/api/v1/policies",
                json={"agent_id": agent["id"], "connector": "github", "tool": tool, "mode": mode},
            )
            assert response.status_code == 200, response.text

        policy("issues.list", "allow")
        policy("issues.create", "ask")
        yield SimpleNamespace(
            app=app,
            settings=settings,
            client=client,
            origin=origin,
            registry=registry,
            policy=policy,
            agent=agent,
            credential=credential,
            human=human,
            adapter=adapter,
            calls=provider_calls,
            provider_status=provider_status,
            data_dir=tmp_path / "data",
        )
    server.should_exit = True
    thread.join(timeout=10)
    assert not thread.is_alive()
