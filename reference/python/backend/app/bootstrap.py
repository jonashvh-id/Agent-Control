from dataclasses import dataclass

from backend.app.core.database.connection import Database
from backend.app.modules.agents.service import AgentService
from backend.app.modules.approvals.service import ApprovalService
from backend.app.modules.audit.repository import AuditRepository
from backend.app.modules.connectors.registry import ConnectorRegistry
from backend.app.modules.gateway.service import GatewayService
from backend.app.modules.identity.service import IdentityService
from backend.app.modules.lifecycle.service import LifecycleService
from backend.app.modules.policies.service import PolicyService


@dataclass
class Services:
    settings: object
    database: Database
    audit: AuditRepository
    identity: IdentityService
    agents: AgentService
    policies: PolicyService
    approvals: ApprovalService
    lifecycle: LifecycleService
    registry: ConnectorRegistry
    gateway: GatewayService


def compose(settings, registry=None):
    database, audit = Database(settings.data_dir), AuditRepository()
    agents, lifecycle = AgentService(database, audit), LifecycleService(database, audit)
    if registry is None:
        registry = ConnectorRegistry()
        if settings.github:
            from connectors.github.registration import register_github

            register_github(registry, settings.github)
    with database.connect(write=True) as db:
        for connector in {tool.connector for tool in registry.all()}:
            db.execute("INSERT OR IGNORE INTO connector_state VALUES(?,0)", (connector,))
    return Services(
        settings,
        database,
        audit,
        IdentityService(database, settings, audit),
        agents,
        PolicyService(database, audit, agents.repo),
        ApprovalService(database, audit),
        lifecycle,
        registry,
        GatewayService(database, settings, agents, lifecycle, registry, audit),
    )
