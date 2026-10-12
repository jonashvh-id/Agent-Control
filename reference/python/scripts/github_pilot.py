"""Minimal local live pilot. One read, one ASK proposal, explicit operator decision, at most one write.

Uses a new isolated runtime and separate agent/human credentials. The external child client has only
an Agent Control token. GitHub credentials remain in the local gh keychain and are never persisted.
"""

import argparse
import json
import os
import secrets
import socket
import subprocess
import sys
import threading
import time
import uuid
from pathlib import Path

import httpx2
import uvicorn

from backend.app.bootstrap import compose
from backend.app.core.config.settings import Settings
from backend.app.core.security.passwords import hash_password
from backend.app.main import create_app
from backend.app.modules.connectors.github_config import GitHubConfig

REPOSITORY = "jonashvh-id/Agent-Control"
TITLE = "Agent Control — validation MCP et approbation humaine"
BODY = (
    "Issue de validation de la phase 2 d’Agent Control.\n\n"
    "Créée par le connecteur GitHub, après une demande MCP soumise à la politique ASK "
    "et validation humaine de ces paramètres exacts.\n\n"
    "Test limité à ce dépôt ; aucune modification de ZNode."
)
ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description="Minimal live GitHub pilot")
    parser.add_argument(
        "--read-proof", type=Path, help="Reuse a successful read proof instead of contacting GitHub again"
    )
    args = parser.parse_args()
    previous = json.loads(args.read_proof.read_text()) if args.read_proof else None
    if previous and (previous.get("repository") != REPOSITORY or previous.get("live_read") != "passed"):
        raise ValueError("Read proof does not match this repository")
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    origin = f"http://127.0.0.1:{port}"
    data = ROOT / "var" / ("github-pilot-" + uuid.uuid4().hex[:8])
    settings = Settings(
        data,
        origin=origin,
        execution_enabled=True,
        github=GitHubConfig((REPOSITORY,), credential_source="gh-demo"),
    )
    seed = compose(settings)
    password = "Pilot9!" + secrets.token_hex(20)
    # Explicitly isolated development operator; never add roles to an existing user's runtime.
    with seed.database.connect(write=True) as db:
        seed.identity.repo.create(db, "pilot-operator", hash_password(password))
    app = create_app(settings)
    server = uvicorn.Server(
        uvicorn.Config(app, host="127.0.0.1", port=port, log_level="error", access_log=False)
    )
    thread = threading.Thread(target=server.run, daemon=True)
    thread.start()
    try:
        with httpx2.Client(base_url=origin, trust_env=False, timeout=30) as operator:
            for _ in range(100):
                try:
                    if operator.get("/health").status_code == 200:
                        break
                except httpx2.TransportError:
                    pass
                time.sleep(0.05)
            else:
                raise RuntimeError("Pilot server did not start")
            human = operator.post(
                "/api/v1/auth/login",
                headers={"x-control-request": "1"},
                json={"username": "pilot-operator", "password": password},
            ).json()
            password = None
            operator.headers["x-csrf-token"] = human["csrf"]
            agent = operator.post(
                "/api/v1/agents",
                json={"name": "External MCP pilot client", "scopes": ["issues:read", "issues:write"]},
            ).json()
            credential = operator.post(
                f"/api/v1/agents/{agent['id']}/credentials", json={"scopes": ["issues:read", "issues:write"]}
            ).json()
            operator.patch("/api/v1/integrations/github", json={"enabled": True}).raise_for_status()
            for tool, mode in [("issues.list", "allow"), ("issues.create", "ask")]:
                operator.put(
                    "/api/v1/policies",
                    json={"agent_id": agent["id"], "connector": "github", "tool": tool, "mode": mode},
                ).raise_for_status()
            environment = {
                key: value for key, value in os.environ.items() if key in {"PATH", "LANG", "LC_ALL", "TMPDIR"}
            }
            environment["AGENT_CONTROL_AGENT_TOKEN"] = credential["bearer_token"]
            environment.pop("PYTHONPATH", None)

            def call(tool, arguments=None):
                request = data / "client-request.json"
                request.write_text(json.dumps(arguments or {}))
                request.chmod(0o600)
                command = [
                    sys.executable,
                    str(ROOT / "scripts/mcp_issue_client.py"),
                    "--url",
                    origin + "/mcp/",
                    "--tool",
                    tool,
                    "--request-file",
                    str(request),
                ]
                child = subprocess.run(command, env=environment, capture_output=True, text=True, timeout=45)
                if child.returncode:
                    raise RuntimeError("External MCP client failed; raw output omitted")
                return json.loads(child.stdout)

            discovered = call("discover")
            if previous is None:
                read = call(
                    "github.issues.list",
                    {
                        "parameters": {"repository": REPOSITORY, "limit": 1},
                        "idempotency_key": "live-list-" + uuid.uuid4().hex,
                    },
                )
                if read["is_error"]:
                    print(
                        json.dumps({"live_read": "failed", "error": read["result"].get("error")}), flush=True
                    )
                    return
            request = {
                "parameters": {"repository": REPOSITORY, "title": TITLE, "body": BODY},
                "idempotency_key": "live-issue-" + uuid.uuid4().hex,
            }
            proposal = call("github.issues.create", request)["result"]
            if proposal.get("status") != "requires_approval":
                raise RuntimeError("Pilot did not require human approval")
            approval_id = proposal["approval_id"]
            proof = {
                "repository": REPOSITORY,
                "tools": discovered["tools"],
                "live_read": "passed",
                "status": "awaiting_human_approval",
                "approval_id": approval_id,
                "title": TITLE,
                "body": BODY,
                "runtime": str(data),
                "prior_read_proof": str(args.read_proof) if previous else None,
            }
            print(json.dumps(proof, ensure_ascii=False), flush=True)
            # No automatic approval: a local human operator must type the exact approval ID.
            decision = sys.stdin.readline().strip()
            if decision != "approve " + approval_id:
                operator.post(
                    f"/api/v1/approvals/{approval_id}/decision", json={"decision": "denied"}
                ).raise_for_status()
                proof["status"] = "denied_without_external_write"
            else:
                operator.post(
                    f"/api/v1/approvals/{approval_id}/decision", json={"decision": "approved"}
                ).raise_for_status()
                request["approval_id"] = approval_id
                executed = call("github.issues.create", request)
                if executed["is_error"]:
                    proof.update(status="execution_failed", error=executed["result"].get("error"))
                else:
                    proof.update(status=executed["result"]["status"], receipt=executed["result"]["receipt"])
                # Revoke the agent, then verify rejection without another provider call.
                operator.delete(
                    f"/api/v1/agents/{agent['id']}/credentials/{credential['id']}"
                ).raise_for_status()
                denied = operator.post(
                    "/api/v1/gateway/execute",
                    headers={"authorization": "Bearer " + credential["bearer_token"]},
                    json={
                        "connector": "github",
                        "tool": "issues.list",
                        "parameters": {"repository": REPOSITORY},
                        "idempotency_key": "after-revocation-" + uuid.uuid4().hex,
                    },
                )
                proof["revocation_http_status"] = denied.status_code
            operator.delete(f"/api/v1/agents/{agent['id']}/credentials/{credential['id']}")
            proof["audit"] = operator.get("/api/v1/audit").json()
            destination = ROOT / "docs/reviews/github-live-proof.json"
            destination.write_text(json.dumps(proof, indent=2, ensure_ascii=False) + "\n")
            print(
                json.dumps(
                    {
                        "status": proof["status"],
                        "receipt": proof.get("receipt"),
                        "revocation_http_status": proof.get("revocation_http_status"),
                        "proof": str(destination),
                    },
                    ensure_ascii=False,
                ),
                flush=True,
            )
    finally:
        server.should_exit = True
        thread.join(timeout=10)


if __name__ == "__main__":
    main()
