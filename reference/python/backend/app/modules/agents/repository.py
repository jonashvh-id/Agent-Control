import json
import uuid

from backend.app.core.errors.domain import DomainError
from backend.app.modules.audit.repository import now


class AgentRepository:
    def get(self, db, agent_id):
        row = db.execute("SELECT * FROM agents WHERE id=?", (agent_id,)).fetchone()
        if not row:
            raise DomainError("agent_not_found", 404)
        return {**dict(row), "scopes": json.loads(row["scopes"])}

    def list(self, db):
        return [self.get(db, r["id"]) for r in db.execute("SELECT id FROM agents ORDER BY created_at DESC")]

    def create(self, db, actor_id, values):
        agent_id, timestamp = uuid.uuid4().hex, now()
        db.execute(
            """INSERT INTO agents(id,name,description,provider,runtime,scopes,created_by,created_at,updated_at)
                   VALUES(?,?,?,?,?,?,?,?,?)""",
            (
                agent_id,
                values.name,
                values.description,
                values.provider,
                values.runtime,
                json.dumps(sorted(set(values.scopes))),
                actor_id,
                timestamp,
                timestamp,
            ),
        )
        return self.get(db, agent_id)

    def issue_credential(self, db, agent_id, token_hash, scopes):
        credential_id = uuid.uuid4().hex
        db.execute(
            "INSERT INTO credentials VALUES(?,?,?,?,?,?,NULL)",
            (credential_id, agent_id, token_hash, json.dumps(sorted(set(scopes))), "active", now()),
        )
        return credential_id

    def resolve(self, db, token_hash):
        row = db.execute(
            "SELECT * FROM credentials WHERE token_hash=? AND status='active'", (token_hash,)
        ).fetchone()
        if not row:
            raise DomainError("agent_authentication_required", 401)
        agent = self.get(db, row["agent_id"])
        if agent["status"] != "active":
            raise DomainError("agent_inactive", 403)
        scopes = set(json.loads(row["scopes"])) & set(agent["scopes"])
        return {"id": agent["id"], "credential_id": row["id"], "scopes": scopes}

    def credentials(self, db, agent_id):
        return [
            dict(r)
            for r in db.execute(
                """SELECT id,agent_id,scopes,status,created_at,revoked_at
                              FROM credentials WHERE agent_id=? ORDER BY created_at DESC""",
                (agent_id,),
            )
        ]
