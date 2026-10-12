from contextlib import asynccontextmanager
from pathlib import Path
from urllib.parse import urlsplit

from fastapi import FastAPI, Request
from fastapi.exceptions import RequestValidationError
from fastapi.responses import FileResponse, JSONResponse
from fastapi.staticfiles import StaticFiles
from starlette.middleware.trustedhost import TrustedHostMiddleware

from backend.app.api.v1 import agents, gateway, governance, identity
from backend.app.bootstrap import compose
from backend.app.core.config.settings import Settings
from backend.app.core.errors.domain import DomainError
from backend.app.core.security.middleware import BoundaryMiddleware


def create_app(settings=None, registry=None):
    settings = settings or Settings.from_env()

    @asynccontextmanager
    async def lifespan(app):
        app.state.services = compose(settings, registry)
        if mcp_app is not None:
            async with mcp_app.router.lifespan_context(mcp_app):
                yield
        else:
            yield

    app = FastAPI(
        title="Agent Control",
        version="0.2.0",
        lifespan=lifespan,
        docs_url=None,
        redoc_url=None,
        openapi_url=None,
    )
    mcp_app = None
    if settings.mcp_enabled:
        from backend.app.modules.mcp.server import build_server

        mcp_app, protected_mcp = build_server(lambda: app.state.services, settings.origin)
        app.mount("/mcp", protected_mcp)
    app.add_middleware(BoundaryMiddleware, origin=settings.origin)
    app.add_middleware(
        TrustedHostMiddleware, allowed_hosts=[urlsplit(settings.origin).hostname, "localhost", "127.0.0.1"]
    )

    @app.exception_handler(DomainError)
    async def domain_error(request: Request, exc: DomainError):
        return JSONResponse({"error": exc.code}, status_code=exc.status)

    @app.exception_handler(RequestValidationError)
    async def validation_error(request: Request, exc: RequestValidationError):
        # FastAPI's default detail echoes invalid inputs (including passwords).
        return JSONResponse({"error": "invalid_request"}, status_code=422)

    @app.get("/health")
    def health():
        with app.state.services.database.connect() as db:
            db.execute("SELECT 1").fetchone()
        return {"status": "ok", "service": "agent-control", "stage": "foundation"}

    for router in [identity.router, agents.router, governance.router, gateway.router]:
        app.include_router(router, prefix="/api/v1")
    # Installed wheels use bundled assets; development uses the repository frontend.
    repository_web = Path(__file__).resolve().parents[2] / "frontend"
    web = repository_web if repository_web.is_dir() else Path(__file__).with_name("web")
    app.mount("/ui", StaticFiles(directory=web), name="ui")

    @app.get("/", include_in_schema=False)
    def index():
        return FileResponse(web / "index.html")

    return app


app = create_app()
