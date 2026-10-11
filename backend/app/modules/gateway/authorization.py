from backend.app.core.errors.domain import DomainError
from backend.app.modules.policies.engine import PolicyEngine
from backend.app.modules.policies.repository import PolicyRepository


class GatewayAuthorization:
    """The single current-authorization path for admission, discovery and replay."""

    def __init__(self, agents, lifecycle, settings):
        self.agents, self.lifecycle, self.settings = agents, lifecycle, settings
        self.policy = PolicyRepository()
        self.engine = PolicyEngine()

    def check(self, db, token, registered):
        actor = self.agents.resolve(db, token)
        self.lifecycle.check(db, registered.connector)
        mode = self.engine.effective_mode(
            tool=registered.definition,
            granted_scopes=actor["scopes"],
            explicit_mode=self.policy.mode(db, actor["id"], registered.connector, registered.definition.name),
        )
        if mode == "deny":
            raise DomainError("policy_denied", 403)
        if not self.settings.execution_enabled:
            raise DomainError("execution_disabled", 503)
        return actor, mode
