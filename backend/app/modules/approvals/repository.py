import json
import time
import uuid

from backend.app.core.errors.domain import DomainError
from backend.app.modules.audit.redaction import summarize_input
from backend.app.modules.audit.repository import now


class ApprovalRepository:
    def __init__(self, secret=None):
        from backend.app.core.security.sealed import PreviewCipher

        self.cipher = PreviewCipher(secret) if secret else None

    def create(self, db, actor, proposal, request_hash, seconds, preview=None):
        pending = db.execute(
            """SELECT id FROM approvals WHERE agent_id=? AND credential_id=? AND input_hash=?
                             AND status='pending' AND expires_at>?""",
            (actor["id"], actor["credential_id"], request_hash, time.time()),
        ).fetchone()
        if pending:
            return pending["id"]
        approval_id = uuid.uuid4().hex
        db.execute(
            """INSERT INTO approvals(id,agent_id,credential_id,connector,tool,input_hash,idempotency_key,
               input_summary,status,expires_at,created_at,preview_ciphertext)
               VALUES(?,?,?,?,?,?,?,?,?,?,?,?)""",
            (
                approval_id,
                actor["id"],
                actor["credential_id"],
                proposal.connector,
                proposal.tool,
                request_hash,
                proposal.idempotency_key,
                json.dumps(summarize_input(proposal.parameters)),
                "pending",
                time.time() + seconds,
                now(),
                self.cipher.seal(preview) if preview is not None else "",
            ),
        )
        return approval_id

    def get(self, db, approval_id):
        row = db.execute("SELECT * FROM approvals WHERE id=?", (approval_id,)).fetchone()
        if not row:
            raise DomainError("approval_not_found", 404)
        result = {**dict(row), "input_summary": json.loads(row["input_summary"])}
        ciphertext = result.pop("preview_ciphertext", "")
        result["input_preview"] = self.cipher.open(ciphertext) if ciphertext and self.cipher else None
        return result

    def expire(self, db):
        db.execute(
            "UPDATE approvals SET status='expired' WHERE status IN ('pending','approved') AND expires_at<=?",
            (time.time(),),
        )

    def list(self, db):
        self.expire(db)
        return [
            self.get(db, r["id"])
            for r in db.execute("SELECT id FROM approvals ORDER BY created_at DESC LIMIT 100")
        ]

    def decide(self, db, approval_id, actor_id, decision):
        self.expire(db)
        changed = db.execute(
            """UPDATE approvals SET status=?,decided_by=?,decided_at=?
                             WHERE id=? AND status='pending' AND expires_at>?""",
            (decision, actor_id, now(), approval_id, time.time()),
        ).rowcount
        if not changed:
            self.get(db, approval_id)
            raise DomainError("approval_not_pending", 409)
        return self.get(db, approval_id)

    def consume(self, db, approval_id, actor, proposal, request_hash):
        approval = self.get(db, approval_id)
        approver = db.execute("SELECT enabled FROM humans WHERE id=?", (approval["decided_by"],)).fetchone()
        if not approver or not approver["enabled"]:
            raise DomainError("approval_authority_expired", 403)
        if (
            approval["agent_id"] != actor["id"]
            or approval["credential_id"] != actor["credential_id"]
            or approval["connector"] != proposal.connector
            or approval["tool"] != proposal.tool
            or approval["input_hash"] != request_hash
            or approval["idempotency_key"] != proposal.idempotency_key
        ):
            raise DomainError("approval_binding_mismatch", 403)
        changed = db.execute(
            """UPDATE approvals SET status='consumed' WHERE id=? AND status='approved'
                             AND expires_at>?""",
            (approval_id, time.time()),
        ).rowcount
        if not changed:
            raise DomainError("approval_unusable", 409)
