import json
import uuid
from datetime import UTC, datetime

from .redaction import summarize_input


def now():
    return datetime.now(UTC).isoformat()


class AuditRepository:
    def record(
        self,
        db,
        *,
        actor_kind,
        actor_id,
        action,
        decision,
        correlation_id=None,
        agent_id="",
        connector="",
        tool="",
        approval_id="",
        execution_id="",
        input_hash="",
        parameters=None,
        human_requester_id="",
        result_summary=None,
    ):
        values = (
            uuid.uuid4().hex,
            now(),
            actor_kind,
            actor_id,
            human_requester_id,
            agent_id,
            connector,
            tool,
            action,
            decision,
            approval_id,
            execution_id,
            correlation_id or uuid.uuid4().hex,
            input_hash,
            json.dumps(summarize_input(parameters) if parameters is not None else {}),
            json.dumps(result_summary or {}),
        )
        db.execute(
            """INSERT INTO audit_events(id,created_at,actor_kind,actor_id,human_requester_id,
                   agent_id,connector,tool,action,decision,approval_id,execution_id,correlation_id,
                   input_hash,input_summary,result_summary) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)""",
            values,
        )

    def list(self, db, limit=100):
        rows = db.execute("SELECT * FROM audit_events ORDER BY sequence DESC LIMIT ?", (limit,)).fetchall()
        return [
            {
                **dict(row),
                "input_summary": json.loads(row["input_summary"]),
                "result_summary": json.loads(row["result_summary"]),
            }
            for row in rows
        ]
