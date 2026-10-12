# Phase 3 product evidence — 2026-10-12

Scope: real model-generated actions, understandable operator workflows and a second independent provider. Rust remains the primary runtime; four acyclic crates share the same governance engine. This is implementer validation, not an independent audit. No deployment, release or live provider write is part of this phase.

## What makes the product useful, simpler or safer

| Change | Product benefit |
|---|---|
| Codex HTTP MCP configuration and scoped connection flow | Connect an existing agent without giving it native application secrets or human admin credentials. |
| Registered application/tool catalog and capability checkboxes | Select actual available actions without typing opaque comma-separated scopes. |
| Effective access panel and reason descriptions | See who can do what and why ALLOW can still be blocked. |
| Exact approval cards and actor names in audit views | Review the actual operation and understand its result, with minimized stored audit. |
| Independent Google Drive search/read adapter | Apply the same engine to multiple applications without moving provider logic into policy or transport. |
| Separate Google setup guidance | Explain why governance approval and native Google authorization are different; avoid false “connected” claims. |

## Real Codex observation

Installed client: Codex CLI 0.137.0, authenticated through the existing ChatGPT account. The optional pilot launched ephemeral sessions with a private Agent Control credential and only this MCP server. Shell, other apps/plugins/browser tools and multi-agent features were disabled. The model never received a human session or native provider credential.

| Model action | Gateway result | Provider evidence |
|---|---|---|
| Read at most one issue from `jonashvh-id/Agent-Control` | HTTP 200 `executed` | Actual GitHub adapter/native read; no issue creation |
| Search configured Drive document | HTTP 200 `executed` | Explicit isolated Drive fixture |
| Request bounded Drive read | HTTP 202 `requires_approval` | No document read before grant |
| Resubmit identical parameters/key after separate operator decision | HTTP 200 `executed` | Fixture read; no native Google claim |
| Repeat that exact completed request | HTTP 200 `already_executed` | Minimal receipt, no second content read |
| Reconnect after revoking the pilot credential | Required MCP initialization refused; client exit 1, zero tool calls | Independent MCP HTTP check returns 401 `agent_authentication_required` |

The first operator decision used the isolated test human role through the UI; it was not an approval by the model and is not presented as a click by the user. Raw model traces, approval previews and screenshots are private/ignored and not committed. Only summarized statuses are reported here.

## Native Google confirmation

The user completed account sign-in and Google consent in the browser. A temporary Online `drive.readonly` access token was exchanged through Google's own Playground and saved in an owned mode-0600 file under an ignored private directory. No refresh token was issued. One Google Doc was allowlisted; neither its name/ID nor its contents are included in this report. The token's native scope can read all files visible to that Google account, while this server restricts requests to the one configured ID. This pilot scope is not a production credential-isolation solution.

A separate owned native-provider fixture composed the actual GitHub and Google adapters. Codex generated the following actions through MCP:

| Native Google action | Observed result |
|---|---|
| Search the configured document | HTTP 200 `executed`, actual Google metadata |
| Request a read limited to 1,200 characters | HTTP 202 `requires_approval`; no document content returned |
| Resubmit after separate operator API decision | HTTP 200 `executed`, 1,200 actual document characters |
| Repeat identical request/key | HTTP 200 `already_executed`, `result_available=false`, minimized character-count receipt |
| Search with the dedicated native token file temporarily replaced by a synthetic invalid token | HTTP 403 `google_access_denied`, failed execution; model stops without fallback. Valid private token restored afterward. |
| Reconnect after revoking this pilot's Agent Control credential | Required MCP initialization refused; client exit 1, zero model tool calls; independent HTTP check returns 401 |

The separate operator decision used the test administrator API/session and exact preview validation, following the user's authorization to perform the tests. The model never received that session. This demonstrates separation of identities and request binding; it is not a claim that a human physically clicked the native read's decision. The minimized audit contains two successful Google executions (search/read), one replay and one native denial, without file ID, native token, agent token or document content. No Google account permission was revoked during the invalid-token check. No native provider writes were performed. Redacted statuses are in [model-evidence.json](phase3/model-evidence.json).

## Automated validation

The default suite contains 128 Rust tests and 6 frontend checks. Three explicitly optional tests are excluded: Python parity, browser fixture and model/product fixture. Google native HTTP fixtures exercise the actual adapter, including bounded content, Unicode, unsupported types, native denials, redirects, malformed/oversized results, integer bounds, credential rotation/permissions and allowlist validation. Governed adapter tests verify ASK, exact binding, one-time dispatch, replay, revocation and audit minimization.

Multi-application tests verify filtered discovery, connector disablement and inability to use one application's approval for another. Human access diagnostics are checked against current scopes/policies/lifecycle; agent credentials cannot access human catalog/diagnostics, and scope changes require an admin plus CSRF. Existing security, standalone and native MCP tests remain in the suite. The pinned Python comparison passed 28 HTTP observations plus its separate-process MCP SDK workflow after API extensions. Historical migration coverage is not presented as coverage of this new adapter.

## Remaining limits

The first model-driven, multi-application live scenario is demonstrated. Google sign-in still uses an external manual pilot flow; the backend supports only a private short-lived token file, without integrated OAuth/refresh. This needs a product connection flow and dedicated native resource access before operational use. The live observations cover one document and one repository, not every provider edge case.

Before client deployment: MFA, managed secret lifecycle and independent security review remain requirements. Validation is by the implementer. The branch is prepared for product/code review, without automatic merge, release or deployment. [validation.json](phase3/validation.json) records local automated checks and unchanged protected projects; the PR's Ubuntu job validates its candidate revision.
