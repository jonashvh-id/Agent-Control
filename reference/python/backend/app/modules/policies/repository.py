from backend.app.modules.audit.repository import now


class PolicyRepository:
    def mode(self, db, agent_id, connector, tool):
        row = db.execute(
            "SELECT mode FROM policies WHERE agent_id=? AND connector=? AND tool=?",
            (agent_id, connector, tool),
        ).fetchone()
        return row["mode"] if row else "deny"

    def set(self, db, agent_id, connector, tool, mode, actor_id):
        db.execute(
            """INSERT INTO policies VALUES(?,?,?,?,?,?) ON CONFLICT(agent_id,connector,tool)
                   DO UPDATE SET mode=excluded.mode,updated_by=excluded.updated_by,updated_at=excluded.updated_at""",
            (agent_id, connector, tool, mode, actor_id, now()),
        )

    def list(self, db):
        return [dict(r) for r in db.execute("SELECT * FROM policies ORDER BY updated_at DESC")]
