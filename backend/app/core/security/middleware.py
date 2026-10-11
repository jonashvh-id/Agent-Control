from starlette.responses import JSONResponse


class BoundaryMiddleware:
    """Same-origin browser boundary, bounded request body and response hardening."""

    def __init__(self, app, origin):
        self.app, self.origin = app, origin

    async def __call__(self, scope, receive, send):
        if scope["type"] != "http":
            await self.app(scope, receive, send)
            return
        headers = dict(scope["headers"])
        origin = headers.get(b"origin", b"").decode()
        if scope["method"] not in {"GET", "HEAD", "OPTIONS"} and origin and origin != self.origin:
            await JSONResponse({"error": "origin_rejected"}, status_code=403)(scope, receive, send)
            return
        # Buffer a small finite body before invoking JSON/Pydantic parsing.
        parts, size = [], 0
        while True:
            message = await receive()
            if message["type"] == "http.disconnect":
                return
            size += len(message.get("body", b""))
            if size > 32768:
                await JSONResponse({"error": "request_too_large"}, status_code=413)(scope, receive, send)
                return
            parts.append(message.get("body", b""))
            if not message.get("more_body", False):
                break
        delivered = False

        async def body():
            nonlocal delivered
            if not delivered:
                delivered = True
                return {"type": "http.request", "body": b"".join(parts), "more_body": False}
            return await receive()

        async def hardened(message):
            if message["type"] == "http.response.start":
                message.setdefault("headers", []).extend(
                    [
                        (b"x-content-type-options", b"nosniff"),
                        (b"x-frame-options", b"DENY"),
                        (b"referrer-policy", b"no-referrer"),
                        (b"cache-control", b"no-store"),
                        (
                            b"content-security-policy",
                            b"default-src 'self'; script-src 'self'; style-src 'self'; "
                            b"img-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; "
                            b"frame-ancestors 'none'; form-action 'self'",
                        ),
                    ]
                )
            await send(message)

        await self.app(scope, body, hardened)
