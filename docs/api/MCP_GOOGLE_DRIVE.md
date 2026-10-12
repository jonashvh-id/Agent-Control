# Google Drive pilot contract

This adapter exposes two read-only operations through the existing governed MCP/REST gateway. Native Google authentication and Agent Control approval are separate: enabling the adapter or approving a proposal does not create Google access.

| MCP tool | Required agent scope | Parameters | Output |
|---|---|---|---|
| `google-drive.files.search` | `drive:search` | Optional `query` (name substring, ≤120 characters), `limit` (integer 1–20, default 10) | Safe metadata for matching configured files; no content |
| `google-drive.files.read` | `drive:read` | Exact allowlisted `file_id`, optional `max_chars` (integer 1–6000, default 6000) | Safe metadata, bounded text, `truncated` |

Both use the standard MCP wrapper: `parameters`, `idempotency_key`, optional `approval_id`. Unknown fields, unsupported tools, fractional numbers and out-of-allowlist targets are rejected before provider I/O. Search fetches metadata only for the configured IDs and filters names locally; it does not perform a general Drive search. Google Docs are exported to plain text, and `text/plain` files are downloaded. PDFs, Sheets, Slides, folders and binary formats are outside this pilot.

For the demonstration, set search to ALLOW and read to ASK. The encrypted human preview shows effective query/limit/allowlist for search or file ID/character bound for read. The immutable binding uses the exact submitted validated parameters, including whether optional parameters were supplied. The agent resubmits those same arguments, idempotency key and approval ID after a separate human decision. Approval does not itself execute the action. Current authority is checked again at dispatch.

## Native access setup

Use a dedicated Google identity with read access to the test document, and obtain a short-lived OAuth access token with suitable native permissions. Save only the access token in an owned, nonsymlink mode-0600 file outside version control. The adapter rereads it on each native HTTP request. Do not put it in the browser UI, chat, agent environment, MCP arguments or audit.

Set `AGENT_CONTROL_GOOGLE_DRIVE_TOKEN_FILE` to that path and `AGENT_CONTROL_GOOGLE_DRIVE_FILES` to a comma-separated list of 1–20 exact IDs. Restart the owned server and enable Google Drive in Applications. Assign `drive:search`/`drive:read`, issue a matching credential and choose tool policies in Permissions. Missing credentials or disabled execution never confer access.

For a temporary local pilot, Google's [OAuth 2.0 Playground](https://developers.google.com/oauthplayground/) can obtain a token: use `https://www.googleapis.com/auth/drive.readonly`, choose Online access, sign in and review the consent yourself, then exchange the authorization code for a token. Do not include credentials/tokens in a shared Playground link. `drive.readonly` has read/download access to all files visible to that Google account; the platform file allowlist does not reduce the token's native scope. A dedicated identity with limited native access is preferable. [Google's scope documentation](https://developers.google.com/workspace/drive/api/guides/api-specific-auth) describes the scope and `drive.file` alternative; selecting files with an integrated picker is not implemented here.

Agent Control has no integrated Google OAuth sign-in, client registration, refresh-token storage or automatic refresh yet. Expired/revoked credentials and native 401/403/404 are returned as `google_access_denied`; do not convert them to platform approval. The normal client connection uses an Agent Control bearer token, never the native Google token.

## Boundaries

Only fixed `https://www.googleapis.com/drive/v3/` requests are possible. No redirects, proxies or retries are followed. Each native request is limited to 10 seconds and 256 KiB, within the existing gateway deadline. Results remain untrusted data. Audit/replay receipts contain only provider plus character/file counts, not document text, names, IDs, query terms or native secrets. Authorized human previews and live agent results necessarily contain selected parameters/metadata; they are distinct from minimized stored audit.

The adapter uses [Google's get endpoint](https://developers.google.com/workspace/drive/api/reference/rest/v3/files/get) and [text export endpoint](https://developers.google.com/workspace/drive/api/reference/rest/v3/files/export). It has no editing, sharing or deletion tool. Google permissions remain authoritative. Direct Google access outside this gateway is outside Agent Control's governance.
