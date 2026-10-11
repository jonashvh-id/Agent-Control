from backend.app.core.errors.domain import DomainError
from backend.app.core.security.tokens import generate_token, hash_token
from backend.app.modules.audit.repository import now

from .repository import AgentRepository


class AgentService:
    def __init__(self, database, audit):
        self.database, self.audit = database, audit
        self.repo = AgentRepository()

    def create(self, actor, values):
        with self.database.connect(write=True) as db:
            agent = self.repo.create(db, actor["id"], values)
            self.audit.record(
                db,
                actor_kind="human",
                actor_id=actor["id"],
                agent_id=agent["id"],
                action="agent.create",
                decision="allow",
            )
            return agent

    def issue_credential(self, actor, agent_id, scopes):
        token = generate_token()
        with self.database.connect(write=True) as db:
            agent = self.repo.get(db, agent_id)
            if agent["status"] != "active":
                raise DomainError("agent_inactive", 409)
            if not set(scopes).issubset(set(agent["scopes"])):
                raise DomainError("credential_scope_escalation", 403)
            credential_id = self.repo.issue_credential(
                db, agent_id, hash_token(token, self.database.secret), scopes
            )
            self.audit.record(
                db,
                actor_kind="human",
                actor_id=actor["id"],
                agent_id=agent_id,
                action="credential.issue",
                decision="allow",
            )
        return {"id": credential_id, "bearer_token": token, "display_once": True}

    def revoke(self, actor, agent_id, credential_id):
        with self.database.connect(write=True) as db:
            self.repo.get(db, agent_id)
            changed = db.execute(
                """UPDATE credentials SET status='revoked',revoked_at=?
                                 WHERE id=? AND agent_id=? AND status='active'""",
                (now(), credential_id, agent_id),
            ).rowcount
            if not changed:
                raise DomainError("active_credential_not_found", 404)
            self.audit.record(
                db,
                actor_kind="human",
                actor_id=actor["id"],
                agent_id=agent_id,
                action="credential.revoke",
                decision="allow",
            )

    def authenticate(self, token):
        with self.database.connect() as db:
            return self.resolve(db, token)

    def resolve(self, db, token):
        return self.repo.resolve(db, hash_token(token, self.database.secret))

    def record_authentication_denial(self, token, action):
        """Audit only verified stored identifiers, never the presented credential."""
        with self.database.connect(write=True) as db:
            credential = db.execute(
                "SELECT id,agent_id FROM credentials WHERE token_hash=?",
                (hash_token(token or "", self.database.secret),),
            ).fetchone()
            self.audit.record(
                db,
                actor_kind="agent" if credential else "unknown",
                actor_id=credential["id"] if credential else "",
                agent_id=credential["agent_id"] if credential else "",
                action=action,
                decision="deny",
            )
