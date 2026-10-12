import sqlite3
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import replace

import pytest
from fastapi.testclient import TestClient

from backend.app.main import create_app
from backend.app.modules.gateway.models import ExecutionProposal
from backend.app.modules.policies.engine import PolicyEngine


def approve(h):
    h.policy("ask")
    response = h.execute()
    assert response.status_code == 202, response.text
    approval_id = response.json()["approval_id"]
    decision = h.client.post(f"/api/v1/approvals/{approval_id}/decision", json={"decision": "approved"})
    assert decision.status_code == 200, decision.text
    return approval_id


def test_default_deny_scope_checks_and_discovery(harness):
    h = harness
    assert h.execute().status_code == 403
    assert h.adapter.calls == []
    assert h.client.get("/api/v1/gateway/tools", headers=h.headers).json()["tools"] == []
    h.policy("allow")
    assert len(h.client.get("/api/v1/gateway/tools", headers=h.headers).json()["tools"]) == 1
    definition = h.registry.all()[0].definition
    assert PolicyEngine().effective_mode(tool=definition, granted_scopes=[], explicit_mode="allow") == "deny"
    assert (
        PolicyEngine().effective_mode(
            tool=replace(definition, requires_confirmation=True),
            granted_scopes=["records:write"],
            explicit_mode="allow",
        )
        == "ask"
    )
    with h.services.database.connect(write=True) as db:
        db.execute("UPDATE agents SET scopes='[]' WHERE id=?", (h.agent["id"],))
    assert h.execute().status_code == 403


def test_unknown_tool_validation_and_request_limits(harness):
    h = harness
    h.policy("allow")
    assert h.execute(tool="unknown.tool").status_code == 404
    assert h.execute(parameters={"name": "x", "unsafe": "unexpected"}).status_code == 422
    assert h.execute(parameters={"name": "x" * 20000}).status_code == 422
    assert h.execute(parameters={"name": "x" * 40000}).status_code == 413
    assert h.execute(human_requester_id=h.actor["id"]).status_code == 422
    assert h.adapter.calls == []


def test_approval_bound_to_immutable_request(harness):
    h = harness
    approval_id = approve(h)
    assert h.adapter.calls == []
    assert h.execute(approval_id=approval_id, parameters={"name": "altered"}).status_code == 403
    assert h.execute(approval_id=approval_id, idempotency_key="different-key").status_code == 403
    assert h.execute(approval_id=approval_id).status_code == 200
    assert h.execute(approval_id=approval_id).json()["status"] == "already_executed"
    assert len(h.adapter.calls) == 1
    with h.services.database.connect(write=True) as db:
        with pytest.raises(sqlite3.IntegrityError):
            db.execute("UPDATE approvals SET input_hash='tampered' WHERE id=?", (approval_id,))


def test_pending_requests_deduplicate_and_decisions_cannot_replay(harness):
    h = harness
    h.policy("ask")
    first, second = h.execute(), h.execute()
    assert first.json()["approval_id"] == second.json()["approval_id"]
    approval_id = first.json()["approval_id"]
    assert (
        h.client.post(f"/api/v1/approvals/{approval_id}/decision", json={"decision": "denied"}).status_code
        == 200
    )
    assert (
        h.client.post(f"/api/v1/approvals/{approval_id}/decision", json={"decision": "approved"}).status_code
        == 409
    )
    assert h.execute(approval_id=approval_id).status_code == 409
    assert not h.adapter.calls


@pytest.mark.parametrize("mode", ["deny", "allow"])
def test_approved_request_revalidates_current_policy(harness, mode):
    h = harness
    approval_id = approve(h)
    h.policy(mode)
    result = h.execute(approval_id=approval_id)
    assert result.status_code == (403 if mode == "deny" else 200)
    assert len(h.adapter.calls) == (0 if mode == "deny" else 1)


def test_approval_expiry(harness, monkeypatch):
    h = harness
    approval_id = approve(h)
    future = time.time() + 1000
    monkeypatch.setattr(time, "time", lambda: future)
    assert h.execute(approval_id=approval_id).status_code == 409
    assert not h.adapter.calls
    approvals = h.client.get("/api/v1/approvals").json()
    assert next(a for a in approvals if a["id"] == approval_id)["status"] == "expired"


def test_approver_revocation(harness):
    h = harness
    approval_id = approve(h)
    with h.services.database.connect(write=True) as db:
        db.execute("UPDATE humans SET enabled=0 WHERE id=?", (h.actor["id"],))
    assert h.execute(approval_id=approval_id).status_code == 403
    assert not h.adapter.calls


def test_idempotency_survives_restart_and_revalidates_replay(harness):
    h = harness
    h.policy("allow")
    first = h.execute()
    assert first.status_code == 200, first.text
    assert h.execute(parameters={"name": "different"}).status_code == 409
    restarted = create_app(h.settings, h.registry)
    with TestClient(restarted, base_url="http://127.0.0.1:8010") as client:
        response = client.post("/api/v1/gateway/execute", json=h.request, headers=h.headers)
        assert response.json()["status"] == "already_executed"
        assert "Sensitive customer name" not in response.text
    h.policy("deny")
    assert h.execute().status_code == 403
    assert len(h.adapter.calls) == 1


def test_uncertain_adapter_failure_never_retries_or_leaks(harness):
    h = harness
    h.policy("allow")
    h.adapter.failure = True
    assert h.execute().status_code == 502
    assert h.execute().status_code == 409
    assert len(h.adapter.calls) == 1
    assert "must-never-be-exposed" not in h.client.get("/api/v1/audit").text


def test_concurrent_execution_claims_only_dispatch_once(harness):
    h = harness
    h.policy("allow")
    proposal = ExecutionProposal(**h.request)
    with ThreadPoolExecutor(max_workers=2) as pool:
        results = list(
            pool.map(lambda _: h.services.gateway.execute(h.credential["bearer_token"], proposal), range(2))
        )
    assert len(h.adapter.calls) == 1
    assert {r.body["status"] for r in results} <= {"executed", "already_executed", "execution_uncertain"}


@pytest.mark.parametrize("control", ["paused", "disabled", "credential", "connector", "global"])
def test_emergency_and_revocation_enforced_server_side(harness, control):
    h = harness
    h.policy("allow")
    if control in {"paused", "disabled"}:
        h.services.lifecycle.set_agent(h.actor, h.agent["id"], control)
    elif control == "credential":
        h.services.agents.revoke(h.actor, h.agent["id"], h.credential["id"])
    elif control == "connector":
        h.services.lifecycle.set_connector(h.actor, "test", False)
    else:
        h.services.lifecycle.stop(h.actor, True)
    assert h.execute().status_code in {401, 403}
    assert h.adapter.calls == []


def test_scoped_credentials_cannot_escalate(harness):
    h = harness
    assert (
        h.client.post(
            f"/api/v1/agents/{h.agent['id']}/credentials", json={"scopes": ["admin:write"]}
        ).status_code
        == 403
    )
    credential = h.client.post(f"/api/v1/agents/{h.agent['id']}/credentials", json={"scopes": []}).json()
    h.headers = {"authorization": "Bearer " + credential["bearer_token"]}
    h.policy("allow")
    assert h.execute().status_code == 403


def test_audit_omits_values_and_cannot_be_mutated(harness):
    h = harness
    h.policy("allow")
    h.execute()
    audit = h.client.get("/api/v1/audit")
    assert "Sensitive customer name" not in audit.text
    assert h.credential["bearer_token"] not in audit.text
    event = next(e for e in audit.json() if e["action"] == "execution.executed")
    assert event["agent_id"] == h.agent["id"] and event["correlation_id"] and event["input_hash"]
    with h.services.database.connect(write=True) as db:
        with pytest.raises(sqlite3.IntegrityError):
            db.execute("DELETE FROM audit_events")


def test_execution_disabled_even_with_allow(harness):
    h = harness
    h.policy("allow")
    disabled = create_app(replace(h.settings, execution_enabled=False), h.registry)
    with TestClient(disabled, base_url="http://127.0.0.1:8010") as client:
        assert client.post("/api/v1/gateway/execute", json=h.request, headers=h.headers).status_code == 503
    assert h.adapter.calls == []


def test_policy_tightening_between_claim_and_dispatch(harness, monkeypatch):
    h = harness
    h.policy("allow")
    claim = h.services.gateway.executions.claim

    def tighten(db, *args, **kwargs):
        execution_id = claim(db, *args, **kwargs)
        db.execute("UPDATE policies SET mode='ask'")
        return execution_id

    monkeypatch.setattr(h.services.gateway.executions, "claim", tighten)
    assert h.execute().status_code == 403
    assert not h.adapter.calls


def test_approval_cannot_transfer_to_another_credential(harness):
    h = harness
    approval_id = approve(h)
    credential = h.client.post(
        f"/api/v1/agents/{h.agent['id']}/credentials", json={"scopes": ["records:write"]}
    ).json()
    h.headers = {"authorization": "Bearer " + credential["bearer_token"]}
    assert h.execute(approval_id=approval_id).status_code == 403
    assert not h.adapter.calls


def test_denied_request_retains_authenticated_agent_identity(harness):
    h = harness
    assert h.execute().status_code == 403
    event = h.client.get("/api/v1/audit").json()[0]
    assert event["action"] == "policy_denied"
    assert event["agent_id"] == h.agent["id"]
    assert event["actor_id"] == h.credential["id"]


def test_connector_invalid_result_is_not_success(harness, monkeypatch):
    h = harness
    h.policy("allow")
    monkeypatch.setattr(h.adapter, "execute", lambda *a, **kw: {"recorded": "private-result"})
    result = h.execute()
    assert result.status_code == 502
    assert result.json()["error"] == "invalid_connector_result"
    assert "private-result" not in result.text
    assert h.execute().status_code == 409


def test_revoked_authentication_is_audited_without_persisting_presented_token(harness):
    h = harness
    h.services.agents.revoke(h.actor, h.agent["id"], h.credential["id"])
    assert h.execute().status_code == 401
    events = h.client.get("/api/v1/audit").json()
    assert events[0]["action"] == "gateway.authentication"
    assert events[0]["actor_id"] == h.credential["id"] and events[0]["agent_id"] == h.agent["id"]
    assert h.credential["bearer_token"] not in str(events)
