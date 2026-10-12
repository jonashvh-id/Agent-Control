from fastapi import APIRouter, Depends, Request
from fastapi.responses import JSONResponse

from backend.app.modules.gateway.models import ExecutionProposal

from .dependencies import agent_token, services

router = APIRouter(prefix="/gateway", tags=["Agent gateway"], dependencies=[Depends(agent_token)])


@router.get("/tools")
def discover(request: Request, token=Depends(agent_token)):
    return {
        "tools": services(request).gateway.discover(token),
        "mcp_transport": "streamable-http" if services(request).settings.mcp_enabled else "disabled",
    }


@router.post("/execute")
def execute(proposal: ExecutionProposal, request: Request, token=Depends(agent_token)):
    outcome = services(request).gateway.execute(token, proposal)
    return JSONResponse(outcome.body, status_code=outcome.status_code)
