from .repository import ApprovalRepository


class ApprovalService:
    def __init__(self, database, audit):
        self.database, self.audit = database, audit
        self.repo = ApprovalRepository(database.secret)

    def decide(self, actor, approval_id, decision):
        with self.database.connect(write=True) as db:
            approval = self.repo.decide(db, approval_id, actor["id"], decision)
            self.audit.record(
                db,
                actor_kind="human",
                actor_id=actor["id"],
                agent_id=approval["agent_id"],
                connector=approval["connector"],
                tool=approval["tool"],
                action="approval.decide",
                decision=decision,
                approval_id=approval_id,
                input_hash=approval["input_hash"],
            )
            return approval
