from fastapi import APIRouter, Depends, Request

from .dependencies import administrator, services
from .schemas import AgentCreate, AgentStatus, CredentialCreate

router = APIRouter(prefix="/agents", tags=["Agent registry"], dependencies=[Depends(administrator)])


@router.get("")
def list_agents(request: Request):
    app = services(request)
    with app.database.connect() as db:
        return app.agents.repo.list(db)


@router.post("", status_code=201)
def create(values: AgentCreate, request: Request, actor=Depends(administrator)):
    return services(request).agents.create(actor, values)


@router.get("/{agent_id}")
def details(agent_id: str, request: Request):
    app = services(request)
    with app.database.connect() as db:
        return {**app.agents.repo.get(db, agent_id), "credentials": app.agents.repo.credentials(db, agent_id)}


@router.patch("/{agent_id}/status")
def status(agent_id: str, values: AgentStatus, request: Request, actor=Depends(administrator)):
    services(request).lifecycle.set_agent(actor, agent_id, values.status)
    return {"status": values.status}


@router.post("/{agent_id}/credentials", status_code=201)
def issue(agent_id: str, values: CredentialCreate, request: Request, actor=Depends(administrator)):
    return services(request).agents.issue_credential(actor, agent_id, values.scopes)


@router.delete("/{agent_id}/credentials/{credential_id}", status_code=204)
def revoke(agent_id: str, credential_id: str, request: Request, actor=Depends(administrator)):
    services(request).agents.revoke(actor, agent_id, credential_id)
