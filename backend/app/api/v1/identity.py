from fastapi import APIRouter, Depends, Request, Response

from backend.app.core.errors.domain import DomainError

from .dependencies import COOKIE_NAME, human, services
from .schemas import Login

router = APIRouter(prefix="/auth", tags=["Human identity"])


@router.post("/login")
def login(values: Login, request: Request, response: Response):
    # Custom header plus same-origin middleware blocks cross-origin browser login submission.
    if request.headers.get("x-control-request") != "1":
        raise DomainError("same_origin_header_required", 403)
    app = services(request)
    token, actor = app.identity.login(
        values.username, values.password, request.client.host if request.client else ""
    )
    response.set_cookie(
        COOKIE_NAME,
        token,
        httponly=True,
        secure=app.settings.secure_cookies,
        samesite="strict",
        max_age=app.settings.session_seconds,
        path="/",
    )
    return actor


@router.get("/me")
def me(actor=Depends(human)):
    return actor


@router.post("/logout", status_code=204)
def logout(request: Request, response: Response, actor=Depends(human)):
    services(request).identity.logout(request.cookies.get(COOKIE_NAME, ""), actor)
    response.delete_cookie(COOKIE_NAME, path="/")
