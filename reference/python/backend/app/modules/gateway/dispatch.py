"""Commit final admission before bounded external I/O; persist outcome separately."""

import time

from jsonschema import Draft202012Validator

from backend.app.core.errors.domain import DomainError
from backend.app.core.security.fingerprint import canonical_json
from backend.app.modules.audit.repository import now

from .models import GatewayOutcome


class GatewayDispatch:
    def __init__(self, database, authorization, approvals, executions, journal):
        self.database, self.authorization, self.approvals, self.executions, self.journal = (
            database,
            authorization,
            approvals,
            executions,
            journal,
        )

    def execute(self, token, proposal, correlation, prepared):
        actor, mode = prepared.actor, prepared.mode
        try:
            with self.database.connect(write=True) as db:
                actor, mode = self.authorization.check(db, token, prepared.registered)
                if mode == "ask" and not prepared.approval_id:
                    raise DomainError("approval_required_after_policy_change", 403)
                if prepared.approval_id:
                    approval = self.approvals.get(db, prepared.approval_id)
                    approver = db.execute(
                        "SELECT enabled FROM humans WHERE id=?", (approval["decided_by"],)
                    ).fetchone()
                    if (
                        approval["status"] != "consumed"
                        or approval["expires_at"] <= time.time()
                        or not approver
                        or not approver["enabled"]
                    ):
                        raise DomainError("approval_authority_expired", 403)
                if not self.executions.start_dispatch(db, prepared.execution_id):
                    raise DomainError("execution_already_dispatched", 409)
                self.journal.event(
                    db,
                    actor,
                    proposal,
                    correlation,
                    mode,
                    "execution.dispatched",
                    prepared.request_hash,
                    prepared.approval_id,
                    prepared.execution_id,
                )
            # No database transaction remains open while the connector is on the network.
            result = prepared.registered.adapter.execute(
                proposal.tool, proposal.parameters, idempotency_key=prepared.execution_id
            )
            if len(canonical_json(result).encode()) > 65536:
                raise DomainError("connector_result_too_large", 502)
            if not Draft202012Validator(prepared.registered.definition.output_schema).is_valid(result):
                raise DomainError("invalid_connector_result", 502)
            receipt_provider = getattr(prepared.registered.adapter, "receipt", None)
            receipt = receipt_provider(result) if receipt_provider else None
        except DomainError as error:
            return self.fail(actor, proposal, correlation, prepared, error)
        except Exception:
            return self.fail(actor, proposal, correlation, prepared, DomainError("connector_failed", 502))
        with self.database.connect(write=True) as db:
            self.executions.finish(db, prepared.execution_id, "executed", receipt=receipt)
            db.execute("UPDATE agents SET last_used_at=? WHERE id=?", (now(), actor["id"]))
            self.journal.event(
                db,
                actor,
                proposal,
                correlation,
                mode,
                "execution.executed",
                prepared.request_hash,
                prepared.approval_id,
                prepared.execution_id,
                receipt=receipt,
            )
        return GatewayOutcome(
            200,
            {
                "status": "executed",
                "execution_id": prepared.execution_id,
                "result": result,
                "receipt": receipt,
                "result_trust": "untrusted_connector_data",
                "correlation_id": correlation,
            },
        )

    def fail(self, actor, proposal, correlation, prepared, error):
        with self.database.connect(write=True) as db:
            self.executions.finish(db, prepared.execution_id, "failed")
            self.journal.event(
                db,
                actor,
                proposal,
                correlation,
                "deny",
                error.code,
                prepared.request_hash,
                prepared.approval_id,
                prepared.execution_id,
            )
        return GatewayOutcome(
            error.status,
            {"error": error.code, "execution_id": prepared.execution_id, "correlation_id": correlation},
        )
