from .repository import PolicyRepository


class PolicyService:
    """Own policy mutations and their audit transaction outside transport routing."""

    def __init__(self, database, audit, agents):
        self.database, self.audit, self.agents = database, audit, agents
        self.repo = PolicyRepository()

    def list(self):
        with self.database.connect() as db:
            return self.repo.list(db)

    def set(self, actor, values):
        with self.database.connect(write=True) as db:
            self.agents.get(db, values.agent_id)
            self.repo.set(db, values.agent_id, values.connector, values.tool, values.mode, actor["id"])
            self.audit.record(
                db,
                actor_kind="human",
                actor_id=actor["id"],
                agent_id=values.agent_id,
                connector=values.connector,
                tool=values.tool,
                action="policy.set",
                decision=values.mode,
            )
