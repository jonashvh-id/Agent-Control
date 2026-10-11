import time

import pytest
from fastapi.testclient import TestClient

from backend.app.core.security.passwords import PasswordError, hash_password, verify_password
from backend.app.main import create_app


def test_password_helper_extracted():
    encoded = hash_password("Independent9!Control")
    assert verify_password("Independent9!Control", encoded)
    assert not verify_password("incorrect", encoded)
    assert not verify_password("Independent9!Control", "broken")
    with pytest.raises(PasswordError):
        hash_password("short")


def test_human_and_agent_boundaries(harness):
    h = harness
    with TestClient(create_app(h.settings, h.registry), base_url="http://127.0.0.1:8010") as other:
        assert other.get("/api/v1/agents", headers=h.headers).status_code == 401
        assert (
            other.post(
                "/api/v1/auth/login", json={"username": "admin", "password": "Foundation9!Secure"}
            ).status_code
            == 403
        )
    assert h.client.get("/api/v1/gateway/tools").status_code == 401
    assert (
        h.client.get(
            "/api/v1/gateway/tools", headers={"authorization": "Bearer " + h.client.cookies.get("ac_session")}
        ).status_code
        == 401
    )


def test_csrf_origin_and_validation_do_not_echo_password(harness):
    h = harness
    assert (
        h.client.post("/api/v1/agents", json={"name": "x"}, headers={"x-csrf-token": "invalid"}).status_code
        == 403
    )
    assert (
        h.client.post(
            "/api/v1/agents", json={"name": "x"}, headers={"origin": "https://evil.example"}
        ).status_code
        == 403
    )
    response = h.client.post(
        "/api/v1/auth/login",
        json={"username": "x", "password": "private" * 100},
        headers={"x-control-request": "1"},
    )
    assert response.status_code == 422
    assert "private" not in response.text
    assert "unsafe-inline" not in h.client.get("/").headers["content-security-policy"]


def test_login_attempts_persist_and_lock(harness):
    h = harness
    for _ in range(5):
        assert (
            h.client.post(
                "/api/v1/auth/login",
                json={"username": "unknown", "password": "incorrect"},
                headers={"x-control-request": "1"},
            ).status_code
            == 401
        )
    assert (
        h.client.post(
            "/api/v1/auth/login",
            json={"username": "unknown", "password": "incorrect"},
            headers={"x-control-request": "1"},
        ).status_code
        == 429
    )


def test_reviewer_cannot_administer(harness):
    h = harness
    response = h.client.post(
        "/api/v1/auth/login",
        json={"username": "reviewer", "password": "Foundation9!Review"},
        headers={"x-control-request": "1"},
    )
    h.client.headers["x-csrf-token"] = response.json()["csrf"]
    assert h.client.get("/api/v1/agents").status_code == 403
    assert (
        h.client.put(
            "/api/v1/policies",
            json={"agent_id": h.agent["id"], "connector": "test", "tool": "records.create", "mode": "allow"},
        ).status_code
        == 403
    )
    assert h.client.get("/api/v1/approvals").status_code == 200


def test_expiry_and_logout_enforced(harness):
    h = harness
    assert h.client.post("/api/v1/auth/logout").status_code == 204
    assert h.client.get("/api/v1/auth/me").status_code == 401
    response = h.client.post(
        "/api/v1/auth/login",
        json={"username": "admin", "password": "Foundation9!Secure"},
        headers={"x-control-request": "1"},
    )
    assert response.status_code == 200
    with h.services.database.connect(write=True) as db:
        db.execute("UPDATE sessions SET expires_at=?", (time.time() - 1,))
    assert h.client.get("/api/v1/auth/me").status_code == 401
