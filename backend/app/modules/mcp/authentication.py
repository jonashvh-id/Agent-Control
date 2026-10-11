"""Every MCP HTTP request rechecks agent identity; cookies and transport state never grant access."""

import re

from starlette.responses import JSONResponse

from backend.app.core.errors.domain import DomainError

BEARER = re.compile(r"^Bearer ([^\s]+)$", re.IGNORECASE)


def token_from(headers):
    match = BEARER.fullmatch(headers.get("authorization", ""))
    if not match:
        raise DomainError("agent_authentication_required", 401)
    return match.group(1)


class MCPAuthentication:
    def __init__(self, app, get_services, origin):
        self.app, self.get_services, self.origin = app, get_services, origin

    async def __call__(self, scope, receive, send):
        if scope["type"] != "http":
            await self.app(scope, receive, send)
            return
        headers = {key.decode().lower(): value.decode() for key, value in scope["headers"]}
        if headers.get("origin") and headers["origin"] != self.origin:
            await JSONResponse({"error": "origin_rejected"}, status_code=403)(scope, receive, send)
            return
        token = ""
        try:
            if sum(key.lower() == b"authorization" for key, _ in scope["headers"]) != 1:
                raise DomainError("agent_authentication_required", 401)
            token = token_from(headers)
            self.get_services().agents.authenticate(token)
        except DomainError as error:
            self.get_services().agents.record_authentication_denial(token, "mcp.authentication")
            await JSONResponse(
                {"error": error.code}, status_code=error.status, headers={"WWW-Authenticate": "Bearer"}
            )(scope, receive, send)
            return
        await self.app(scope, receive, send)
