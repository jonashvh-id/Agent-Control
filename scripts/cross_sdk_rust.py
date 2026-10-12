"""Migration-only SDK interoperability check against a Rust fixture server; no live GitHub I/O."""
import asyncio
import json
import os

import httpx2
from mcp import ClientSession
from mcp.client.streamable_http import streamable_http_client


async def main():
    origin = os.environ["AGENT_CONTROL_RUST_TEST_ORIGIN"]
    async with httpx2.AsyncClient(base_url=origin, trust_env=False) as human:
        login = await human.post("/api/v1/auth/login", headers={"x-control-request": "1"}, json={"username": "admin", "password": "Foundation9!Secure"})
        login.raise_for_status()
        human.headers["x-csrf-token"] = login.json()["csrf"]
        agent = (await human.post("/api/v1/agents", json={"name": "Cross SDK fixture", "scopes": ["issues:read", "issues:write"]})).json()
        aid = agent["id"]
        credential = (await human.post(f"/api/v1/agents/{aid}/credentials", json={"scopes": ["issues:read", "issues:write"]})).json()
        token = credential["bearer_token"]
        for tool, mode in [("issues.list", "allow"), ("issues.create", "allow")]:
            response = await human.put("/api/v1/policies", json={"agent_id": aid, "connector": "github", "tool": tool, "mode": mode})
            response.raise_for_status()
        async with httpx2.AsyncClient(headers={"authorization": "Bearer " + token}, trust_env=False) as agent_client:
            async with streamable_http_client(origin + "/mcp/", http_client=agent_client) as streams:
                async with ClientSession(*streams) as session:
                    await session.initialize()
                    tools = await session.list_tools()
                    assert {t.name for t in tools.tools} == {"github.issues.list", "github.issues.create"}
                    read = await session.call_tool("github.issues.list", {"parameters": {"repository": "demo/repo"}, "idempotency_key": "cross-sdk-read-001"})
                    assert read.structured_content["status"] == "executed"
                    args = {"parameters": {"repository": "demo/repo", "title": "Cross SDK fixture title", "body": "Cross SDK fixture body"}, "idempotency_key": "cross-sdk-create-001"}
                    pending = await session.call_tool("github.issues.create", args)
                    assert pending.structured_content["status"] == "requires_approval"
                    approval_id = pending.structured_content["approval_id"]
                    response = await human.post(f"/api/v1/approvals/{approval_id}/decision", json={"decision": "approved"})
                    response.raise_for_status()
                    approved = {**args, "approval_id": approval_id}
                    executed = await session.call_tool("github.issues.create", approved)
                    assert executed.structured_content["status"] == "executed"
                    replay = await session.call_tool("github.issues.create", approved)
                    assert replay.structured_content["status"] == "already_executed"
        response = await human.delete(f"/api/v1/agents/{aid}/credentials/{credential['id']}")
        assert response.status_code == 204
        async with httpx2.AsyncClient(trust_env=False) as client:
            revoked = await client.post(origin + "/mcp/", headers={"authorization": "Bearer " + token}, json={"jsonrpc": "2.0", "id": 1, "method": "tools/list"})
            assert revoked.status_code == 401
    print(json.dumps({"cross_sdk": "Python MCP SDK to Rust Streamable HTTP", "governed_workflow": "passed", "live_provider_writes": 0}))


if __name__ == "__main__":
    asyncio.run(main())
