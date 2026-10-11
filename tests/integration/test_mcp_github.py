import asyncio
import json
import sqlite3
import threading
import time
from concurrent.futures import ThreadPoolExecutor

import httpx2
import pytest
from mcp import ClientSession
from mcp.client.streamable_http import streamable_http_client


async def connect_and(h, action):
    async with httpx2.AsyncClient(
        headers={"Authorization": "Bearer " + h.credential["bearer_token"]}, trust_env=False
    ) as client:
        async with streamable_http_client(h.origin + "/mcp/", http_client=client) as streams:
            async with ClientSession(*streams) as session:
                await session.initialize()
                return await action(session)


def create_arguments(key="mcp-create-001"):
    return {
        "parameters": {
            "repository": "demo/repo",
            "title": "Exact human title",
            "body": "Exact private issue body",
        },
        "idempotency_key": key,
    }


def test_external_sdk_client_discovers_reads_asks_and_executes_exact_approved_issue(github_server):
    h = github_server

    async def scenario(session):
        tools = await session.list_tools()
        assert {t.name for t in tools.tools} == {"github.issues.list", "github.issues.create"}
        read = await session.call_tool(
            "github.issues.list",
            {"parameters": {"repository": "demo/repo"}, "idempotency_key": "list-request-001"},
        )
        assert read.structured_content["status"] == "executed"
        proposal = await session.call_tool("github.issues.create", create_arguments())
        assert proposal.structured_content["status"] == "requires_approval"
        assert len(h.calls) == 1
        approval_id = proposal.structured_content["approval_id"]
        approval = h.client.get("/api/v1/approvals").json()[0]
        assert approval["input_preview"] == {
            "repository": "demo/repo",
            "title": "Exact human title",
            "body": "Exact private issue body",
            "operation": "issues.create",
        }
        # Separate authenticated human management API; MCP cannot decide approvals.
        decision = h.client.post(f"/api/v1/approvals/{approval_id}/decision", json={"decision": "approved"})
        assert decision.status_code == 200
        changed = await session.call_tool(
            "github.issues.create",
            {
                **create_arguments(),
                "parameters": {
                    "repository": "demo/repo",
                    "title": "altered",
                    "body": "Exact private issue body",
                },
                "approval_id": approval_id,
            },
        )
        assert changed.is_error and len(h.calls) == 1
        executed = await session.call_tool(
            "github.issues.create", {**create_arguments(), "approval_id": approval_id}
        )
        assert executed.structured_content["status"] == "executed"
        assert executed.structured_content["receipt"]["issue_number"] == 7
        replay = await session.call_tool(
            "github.issues.create", {**create_arguments(), "approval_id": approval_id}
        )
        assert replay.structured_content["status"] == "already_executed"
        assert len(h.calls) == 2
        assert json.loads(h.calls[-1].content) == {
            "title": "Exact human title",
            "body": "Exact private issue body",
        }

    asyncio.run(connect_and(h, scenario))
    events = h.client.get("/api/v1/audit").json()
    executed = next(e for e in events if e["action"] == "execution.executed" and e["tool"] == "issues.create")
    assert executed["result_summary"]["issue_number"] == 7
    assert executed["approval_id"] and executed["correlation_id"] and executed["agent_id"] == h.agent["id"]
    with sqlite3.connect(h.data_dir / "agent-control.sqlite3") as db:
        raw = str(db.execute("SELECT * FROM approvals").fetchall()) + str(
            db.execute("SELECT * FROM audit_events").fetchall()
        )
        assert "Exact human title" not in raw and "Exact private issue body" not in raw


def test_api_routes_cannot_bypass_mcp_policy_or_mandatory_approval(github_server):
    h = github_server
    h.policy("issues.create", "allow")
    headers = {"authorization": "Bearer " + h.credential["bearer_token"]}
    request = {"connector": "github", "tool": "issues.create", **create_arguments()}
    assert h.client.post("/api/v1/gateway/execute", headers=headers, json=request).status_code == 202
    assert not h.calls
    h.policy("issues.create", "deny")
    assert h.client.post("/api/v1/gateway/execute", headers=headers, json=request).status_code == 403
    assert h.client.post("/api/v1/gateway/execute", json=request).status_code == 401
    delete = {**request, "tool": "repos.delete", "parameters": {"repository": "demo/repo"}}
    assert h.client.post("/api/v1/gateway/execute", headers=headers, json=delete).status_code == 404

    async def hidden(session):
        assert [t.name for t in (await session.list_tools()).tools] == ["github.issues.list"]
        result = await session.call_tool("github.issues.create", create_arguments())
        assert result.is_error
        override = await session.call_tool(
            "github.issues.list", {**create_arguments(), "tool": "issues.create"}
        )
        assert override.is_error

    asyncio.run(connect_and(h, hidden))
    assert not h.calls


@pytest.mark.parametrize("control", ["credential", "agent", "connector", "global", "policy"])
def test_revocation_and_tightening_between_approval_and_dispatch_block_provider(github_server, control):
    h = github_server

    async def scenario(session):
        proposal = await session.call_tool("github.issues.create", create_arguments())
        approval_id = proposal.structured_content["approval_id"]
        h.client.post(f"/api/v1/approvals/{approval_id}/decision", json={"decision": "approved"})
        if control == "credential":
            h.client.delete(f"/api/v1/agents/{h.agent['id']}/credentials/{h.credential['id']}")
        elif control == "agent":
            h.client.patch(f"/api/v1/agents/{h.agent['id']}/status", json={"status": "paused"})
        elif control == "connector":
            h.client.patch("/api/v1/integrations/github", json={"enabled": False})
        elif control == "global":
            h.client.put("/api/v1/security/emergency-stop", json={"stopped": True})
        else:
            h.policy("issues.create", "deny")
        response = h.client.post(
            "/api/v1/gateway/execute",
            headers={"authorization": "Bearer " + h.credential["bearer_token"]},
            json={
                "connector": "github",
                "tool": "issues.create",
                **create_arguments(),
                "approval_id": approval_id,
            },
        )
        assert response.status_code in {401, 403}
        # Every new MCP HTTP request rechecks the same credential, including a forged session header.
        raw = h.client.post(
            "/mcp/",
            headers={
                "authorization": "Bearer " + h.credential["bearer_token"],
                "accept": "application/json, text/event-stream",
                "mcp-session-id": "forged",
            },
            json={"jsonrpc": "2.0", "id": 8, "method": "tools/list"},
        )
        assert raw.status_code in (
            {401} if control == "credential" else {403} if control == "agent" else {200, 400}
        )

    asyncio.run(connect_and(h, scenario))
    assert not h.calls


def test_native_github_denial_cannot_be_overridden_by_human_approval(github_server):
    h = github_server
    h.provider_status["code"] = 403

    async def scenario(session):
        proposal = await session.call_tool("github.issues.create", create_arguments())
        aid = proposal.structured_content["approval_id"]
        h.client.post(f"/api/v1/approvals/{aid}/decision", json={"decision": "approved"})
        denied = await session.call_tool("github.issues.create", {**create_arguments(), "approval_id": aid})
        assert denied.is_error and denied.structured_content["error"] == "github_access_denied"
        again = await session.call_tool("github.issues.create", {**create_arguments(), "approval_id": aid})
        assert again.structured_content["status"] == "execution_uncertain"

    asyncio.run(connect_and(h, scenario))
    assert len(h.calls) == 1


def test_mcp_auth_origin_and_cookies_never_grant_agent_access(github_server):
    h = github_server
    message = {"jsonrpc": "2.0", "id": 1, "method": "tools/list"}
    assert h.client.post("/mcp/", json=message).status_code == 401
    headers = {"authorization": "Bearer " + h.credential["bearer_token"], "origin": "https://evil.example"}
    assert h.client.post("/mcp/", json=message, headers=headers).status_code == 403
    assert h.client.get("/mcp/", headers=headers).status_code == 403
    assert not h.calls


def test_control_updates_are_not_blocked_by_external_io(github_server):
    h = github_server
    entered, released = threading.Event(), threading.Event()
    original = h.adapter.execute

    def slow(*args, **kwargs):
        entered.set()
        assert released.wait(5)
        return original(*args, **kwargs)

    h.adapter.execute = slow

    def execute():
        return h.client.post(
            "/api/v1/gateway/execute",
            headers={"authorization": "Bearer " + h.credential["bearer_token"]},
            json={
                "connector": "github",
                "tool": "issues.list",
                "parameters": {"repository": "demo/repo"},
                "idempotency_key": "slow-request-001",
            },
        )

    with ThreadPoolExecutor(max_workers=1) as pool:
        future = pool.submit(execute)
        assert entered.wait(3)
        start = time.monotonic()
        stop = h.client.put("/api/v1/security/emergency-stop", json={"stopped": True})
        assert stop.status_code == 200 and time.monotonic() - start < 2
        released.set()
        assert future.result(timeout=5).status_code == 200
    assert len(h.calls) == 1  # Already admitted I/O may finish; later admission must fail.
    assert execute().status_code == 403
