# API foundation

Base: `/api/v1`. JSON bodies reject unexpected fields. Errors contain stable sanitized `error` codes. Sensitive validation inputs are not echoed. The server has no public OpenAPI/docs endpoint; `openapi.json` in this directory is a static development contract.

## Human API

POST `/auth/login` accepts username/password and requires `X-Control-Request: 1`. It sets an HttpOnly session cookie and returns id, username, role and csrf. GET `/auth/me` reads the current session; POST `/auth/logout` invalidates it. Authenticated mutations require `X-CSRF-Token` from the current session. Browser origin must match the configured origin. Reviewer read/decision access is separate from admin mutation permissions.

| Endpoint | Method | Permission / behavior |
|---|---|---|
| `/agents` | GET / POST | Admin list/create registry identity |
| `/agents/{id}` | GET | Admin metadata and public credential records |
| `/agents/{id}/status` | PATCH | Admin active/paused/disabled |
| `/agents/{id}/credentials` | POST | Admin credential creation; one-time bearer response |
| `/agents/{id}/credentials/{credential_id}` | DELETE | Admin irreversible credential revoke |
| `/policies` | GET / PUT | Human read / admin explicit tool policy |
| `/approvals` | GET | Human queue/history; expires stale grants |
| `/approvals/{id}/decision` | POST | Human approved/denied decision |
| `/audit` | GET | Human latest 100 minimized events |
| `/integrations` | GET | Human registered connector state; empty by default |
| `/integrations/{id}` | PATCH | Admin enable/disable registered trusted connector |
| `/settings` | GET | Human configuration/status and planned capabilities |
| `/security/emergency-stop` | PUT | Admin global admission stop/resume |

## Agent API

Use `Authorization: Bearer <agent credential>`. It grants no human permissions. GET `/gateway/tools` returns tools passing current admission checks (empty in the default runtime).

POST `/gateway/execute` GitHub pilot proposal example:

```json
{"connector":"github","tool":"issues.create","parameters":{"repository":"jonashvh-id/Agent-Control","title":"Reviewed title","body":"Reviewed body"},"idempotency_key":"unique-client-key"}
```

The GitHub pilot adapter is available only with explicit operator configuration, connector activation and policy. Without these conditions this example cannot execute. All writes and reads require an idempotency key of 8–128 safe characters in this foundation. The request body limit is 32 KiB; parameters are capped at 16 KiB, 1000 nodes and depth 32. Tool JSON schema adds stricter validation.

ASK returns HTTP 202 with `status: requires_approval` and an `approval_id`. After an authorized human decision, resend identical connector/tool/parameters/idempotency key with that approval ID. The original raw parameters are not stored or replayed by a worker. A submitted `human_requester_id` is rejected.

Successful dispatch returns `executed`, execution ID and an untrusted connector result. Duplicate success returns `already_executed` and a receipt only. Prepared/failed duplicates return 409 `execution_uncertain`. Policy tightening, revocation or emergency stop blocks replay. The Streamable HTTP MCP transport is available at `/mcp/` and delegates to this same gateway; see `MCP_GITHUB.md`.

GET `/health` is public and returns service health and foundation stage only. SQLite is checked, but this is not external connector or agent health monitoring.
