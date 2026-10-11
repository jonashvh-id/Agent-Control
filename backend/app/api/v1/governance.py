from fastapi import APIRouter, Depends, Request

from .dependencies import administrator, human, services
from .schemas import ApprovalDecision, EnabledSet, PolicySet, StopSet

router = APIRouter(tags=["Governance"], dependencies=[Depends(human)])


@router.get("/policies")
def policies(request: Request):
    return services(request).policies.list()


@router.put("/policies")
def set_policy(values: PolicySet, request: Request, actor=Depends(administrator)):
    app = services(request)
    app.policies.set(actor, values)
    return {
        **values.model_dump(),
        "tool_registered": app.registry.get(values.connector, values.tool) is not None,
    }


@router.get("/approvals")
def approvals(request: Request):
    app = services(request)
    with app.database.connect(write=True) as db:
        return app.approvals.repo.list(db)


@router.post("/approvals/{approval_id}/decision")
def decide(approval_id: str, values: ApprovalDecision, request: Request, actor=Depends(human)):
    return services(request).approvals.decide(actor, approval_id, values.decision)


@router.get("/audit")
def audit(request: Request):
    app = services(request)
    with app.database.connect() as db:
        return app.audit.list(db)


@router.get("/integrations")
def integrations(request: Request):
    app = services(request)
    with app.database.connect() as db:
        return [
            {"id": r["id"], "enabled": bool(r["enabled"]), "status": "registered"}
            for r in db.execute("SELECT * FROM connector_state ORDER BY id")
        ]


@router.patch("/integrations/{connector}")
def connector_status(connector: str, values: EnabledSet, request: Request, actor=Depends(administrator)):
    services(request).lifecycle.set_connector(actor, connector, values.enabled)
    return {"enabled": values.enabled}


@router.get("/settings")
def settings(request: Request):
    app = services(request)
    with app.database.connect() as db:
        return {
            "version": "0.2.0",
            "stage": "github-pilot" if app.settings.github else "foundation",
            "default_policy": "deny",
            "execution_enabled": app.settings.execution_enabled,
            "emergency_stop": app.lifecycle.state(db),
            "external_adapters": len({tool.connector for tool in app.registry.all()}),
            "registered_tools": len(app.registry.all()),
            "mcp_transport": "streamable-http" if app.settings.mcp_enabled else "disabled",
            "cancellation": "planned",
            "health_monitoring": "planned",
            "coverage": "Connections routed through this gateway only",
        }


@router.put("/security/emergency-stop")
def emergency(values: StopSet, request: Request, actor=Depends(administrator)):
    services(request).lifecycle.stop(actor, values.stopped)
    return {"stopped": values.stopped}
