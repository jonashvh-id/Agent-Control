"""Compose shared governance admission/dispatch; REST and MCP use this same interface."""

import uuid

from backend.app.core.errors.domain import DomainError
from backend.app.modules.approvals.repository import ApprovalRepository
from backend.app.modules.execution.repository import ExecutionRepository

from .admission import GatewayAdmission
from .authorization import GatewayAuthorization
from .dispatch import GatewayDispatch
from .journal import GatewayJournal
from .models import GatewayOutcome


class GatewayService:
    def __init__(self, database, settings, agents, lifecycle, registry, audit):
        self.database, self.settings, self.registry = database, settings, registry
        self.authorization = GatewayAuthorization(agents, lifecycle, settings)
        self.approvals, self.executions = ApprovalRepository(database.secret), ExecutionRepository()
        journal = GatewayJournal(audit, registry)
        self.admission = GatewayAdmission(
            database, settings, self.authorization, registry, self.approvals, self.executions, journal
        )
        self.dispatch = GatewayDispatch(
            database, self.authorization, self.approvals, self.executions, journal
        )

    def execute(self, token, proposal):
        proposal = proposal.model_copy(deep=True)
        correlation = uuid.uuid4().hex
        prepared = self.admission.prepare(token, proposal, correlation)
        if isinstance(prepared, GatewayOutcome):
            return prepared
        return self.dispatch.execute(token, proposal, correlation, prepared)

    def discover(self, token):
        tools = []
        with self.database.connect() as db:
            self.authorization.agents.resolve(db, token)
            for registered in self.registry.all():
                try:
                    _, mode = self.authorization.check(db, token, registered)
                except DomainError:
                    continue
                tools.append(
                    {"connector": registered.connector, **registered.definition.to_dict(), "policy": mode}
                )
        return tools
