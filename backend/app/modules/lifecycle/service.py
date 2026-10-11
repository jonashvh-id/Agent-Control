from backend.app.core.errors.domain import DomainError
from backend.app.modules.audit.repository import now


class LifecycleService:
    def __init__(self, database, audit):
        self.database, self.audit = database, audit

    def state(self, db):
        return bool(db.execute("SELECT stopped FROM emergency_state WHERE id=1").fetchone()[0])

    def check(self, db, connector):
        if self.state(db):
            raise DomainError("emergency_stop", 403)
        row = db.execute("SELECT enabled FROM connector_state WHERE id=?", (connector,)).fetchone()
        if not row or not row[0]:
            raise DomainError("connector_disabled", 403)

    def stop(self, actor, stopped):
        with self.database.connect(write=True) as db:
            db.execute("UPDATE emergency_state SET stopped=? WHERE id=1", (int(stopped),))
            self.audit.record(
                db,
                actor_kind="human",
                actor_id=actor["id"],
                action="emergency_stop",
                decision="stopped" if stopped else "resumed",
            )

    def set_agent(self, actor, agent_id, status):
        with self.database.connect(write=True) as db:
            changed = db.execute(
                "UPDATE agents SET status=?,updated_at=? WHERE id=?", (status, now(), agent_id)
            ).rowcount
            if not changed:
                raise DomainError("agent_not_found", 404)
            self.audit.record(
                db,
                actor_kind="human",
                actor_id=actor["id"],
                agent_id=agent_id,
                action="agent.status",
                decision=status,
            )

    def set_connector(self, actor, connector, enabled):
        with self.database.connect(write=True) as db:
            changed = db.execute(
                "UPDATE connector_state SET enabled=? WHERE id=?", (int(enabled), connector)
            ).rowcount
            if not changed:
                raise DomainError("connector_not_found", 404)
            self.audit.record(
                db,
                actor_kind="human",
                actor_id=actor["id"],
                connector=connector,
                action="connector.status",
                decision="enabled" if enabled else "disabled",
            )
