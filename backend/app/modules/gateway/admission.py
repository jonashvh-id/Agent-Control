"""Validate proposals and atomically consume approval / claim execution before I/O."""

from dataclasses import dataclass

from jsonschema import Draft202012Validator

from backend.app.core.errors.domain import DomainError
from backend.app.core.security.fingerprint import fingerprint

from .models import GatewayOutcome


@dataclass(frozen=True)
class PreparedExecution:
    registered: object
    actor: dict
    request_hash: str
    mode: str
    approval_id: str
    execution_id: str


class GatewayAdmission:
    def __init__(self, database, settings, authorization, registry, approvals, executions, journal):
        self.database, self.settings, self.authorization = database, settings, authorization
        self.registry, self.approvals, self.executions, self.journal = (
            registry,
            approvals,
            executions,
            journal,
        )

    def prepare(self, token, proposal, correlation):
        request_hash = fingerprint(proposal.binding(), self.database.secret)
        registered = self.registry.get(proposal.connector, proposal.tool)
        actor, mode = None, "deny"
        approval_id = proposal.approval_id or ""
        with self.database.connect(write=True) as db:
            try:
                actor = self.authorization.agents.resolve(db, token)
                if registered is None:
                    raise DomainError("tool_not_registered", 404)
                actor, mode = self.authorization.check(db, token, registered)
                if not Draft202012Validator(registered.definition.input_schema).is_valid(proposal.parameters):
                    raise DomainError("invalid_tool_parameters", 422)
                validator = getattr(registered.adapter, "validate", None)
                if validator:
                    validator(proposal.tool, proposal.parameters)
                previous = self.executions.previous(db, actor["id"], proposal.idempotency_key, request_hash)
                if previous:
                    self.journal.event(
                        db,
                        actor,
                        proposal,
                        correlation,
                        mode,
                        "execution.replay",
                        request_hash,
                        approval_id,
                        previous["id"],
                    )
                    import json

                    receipt = (
                        json.loads(previous["result_receipt"]) if previous.get("result_receipt") else None
                    )
                    return GatewayOutcome(
                        200 if previous["status"] == "executed" else 409,
                        {
                            "status": "already_executed"
                            if previous["status"] == "executed"
                            else "execution_uncertain",
                            "execution_id": previous["id"],
                            "result_available": False,
                            "receipt": receipt,
                            "correlation_id": correlation,
                        },
                    )
                if approval_id:
                    self.approvals.consume(db, approval_id, actor, proposal, request_hash)
                elif mode == "ask":
                    previewer = getattr(registered.adapter, "preview", None)
                    preview = previewer(proposal.tool, proposal.parameters) if previewer else None
                    approval_id = self.approvals.create(
                        db, actor, proposal, request_hash, self.settings.approval_seconds, preview=preview
                    )
                    self.journal.event(
                        db,
                        actor,
                        proposal,
                        correlation,
                        mode,
                        "approval.requested",
                        request_hash,
                        approval_id,
                    )
                    return GatewayOutcome(
                        202,
                        {
                            "status": "requires_approval",
                            "approval_id": approval_id,
                            "correlation_id": correlation,
                        },
                    )
                execution_id = self.executions.claim(
                    db, actor["id"], proposal.idempotency_key, request_hash, approval_id or None
                )
                self.journal.event(
                    db,
                    actor,
                    proposal,
                    correlation,
                    mode,
                    "execution.prepared",
                    request_hash,
                    approval_id,
                    execution_id,
                )
                return PreparedExecution(registered, actor, request_hash, mode, approval_id, execution_id)
            except DomainError as error:
                self.journal.event(
                    db, actor, proposal, correlation, "deny", error.code, request_hash, approval_id
                )
                return GatewayOutcome(error.status, {"error": error.code, "correlation_id": correlation})
