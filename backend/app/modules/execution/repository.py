import json
import uuid

from backend.app.core.errors.domain import DomainError
from backend.app.modules.audit.repository import now


class ExecutionRepository:
    def previous(self, db, actor_id, key, request_hash):
        row = db.execute(
            "SELECT * FROM executions WHERE agent_id=? AND idempotency_key=?", (actor_id, key)
        ).fetchone()
        if row and row["request_hash"] != request_hash:
            raise DomainError("idempotency_conflict", 409)
        return dict(row) if row else None

    def claim(self, db, actor_id, key, request_hash, approval_id=None):
        execution_id = uuid.uuid4().hex
        db.execute(
            "INSERT INTO executions(id,agent_id,idempotency_key,request_hash,approval_id,status,created_at) VALUES(?,?,?,?,?,?,?)",
            (execution_id, actor_id, key, request_hash, approval_id, "prepared", now()),
        )
        return execution_id

    def start_dispatch(self, db, execution_id):
        return (
            db.execute(
                """UPDATE executions SET dispatch_started_at=? WHERE id=? AND status='prepared'
                          AND dispatch_started_at IS NULL""",
                (now(), execution_id),
            ).rowcount
            == 1
        )

    def finish(self, db, execution_id, status, receipt=None):
        db.execute(
            "UPDATE executions SET status=?,result_receipt=? WHERE id=? AND status='prepared'",
            (status, json.dumps(receipt) if receipt else None, execution_id),
        )
