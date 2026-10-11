"""Minimized governance events, shared by admission and dispatch."""


class GatewayJournal:
    def __init__(self, audit, registry):
        self.audit, self.registry = audit, registry

    def event(
        self,
        db,
        actor,
        proposal,
        correlation,
        mode,
        action,
        request_hash,
        approval_id="",
        execution_id="",
        receipt=None,
    ):
        known = self.registry.get(proposal.connector, proposal.tool) is not None
        self.audit.record(
            db,
            actor_kind="agent" if actor else "unknown",
            actor_id=actor["credential_id"] if actor else "",
            agent_id=actor["id"] if actor else "",
            connector=proposal.connector if known else "",
            tool=proposal.tool if known else "",
            action=action,
            decision=mode,
            correlation_id=correlation,
            input_hash=request_hash,
            parameters=proposal.parameters,
            approval_id=approval_id,
            execution_id=execution_id,
            result_summary=receipt,
        )
