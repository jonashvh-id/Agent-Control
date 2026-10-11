import secrets

from fastapi import Depends, Request
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer

from backend.app.core.errors.domain import DomainError

bearer = HTTPBearer(auto_error=False)
COOKIE_NAME = "ac_session"


def services(request: Request):
    return request.app.state.services


def human(request: Request):
    actor = services(request).identity.authenticate(request.cookies.get(COOKIE_NAME, ""))
    if request.method not in {"GET", "HEAD", "OPTIONS"}:
        csrf = request.headers.get("x-csrf-token", "")
        if not csrf or not secrets.compare_digest(csrf, actor["csrf"]):
            raise DomainError("csrf_required", 403)
    return actor


def administrator(actor=Depends(human)):
    if actor["role"] != "admin":
        raise DomainError("administrator_required", 403)
    return actor


def agent_token(request: Request, credential: HTTPAuthorizationCredentials | None = Depends(bearer)):
    token = credential.credentials if credential else ""
    app = services(request)
    try:
        app.agents.authenticate(token)
    except DomainError:
        app.agents.record_authentication_denial(token, "gateway.authentication")
        raise
    return token
