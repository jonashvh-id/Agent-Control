# Integrated Google connection, resource permissions and operations

This extends the existing Rust architecture. The user selected preparation of the code/configuration flow, with a dedicated Google Reader account; no Google Cloud OAuth client currently exists. Isolated tests exercise the new native OAuth flow. The earlier live Phase 3 file-token read is not evidence of a live integrated OAuth flow.

## One-time operator configuration

1. Create your own Google Cloud project, enable Google Drive API and configure the OAuth consent screen. For a testing app, add the dedicated reader account as a test user. Follow [Google’s Web OAuth guide](https://developers.google.com/identity/protocols/oauth2/web-server).
2. Register an OAuth **Web application** client. In Applications, open **Configure Google OAuth** and copy its exact redirect URI, including the origin/port: `http://127.0.0.1:8010/api/v1/connections/google-drive/callback` for the default local listener. A production origin requires HTTPS and secure cookies. No dynamic return URLs are accepted.
3. Save the client secret alone in an operator-owned nonsymlink file outside the repository. Set mode 0600 and set `AGENT_CONTROL_GOOGLE_OAUTH_CLIENT_ID` and `AGENT_CONTROL_GOOGLE_OAUTH_SECRET_FILE`. The application does not load `.env` automatically. Do not configure legacy `AGENT_CONTROL_GOOGLE_DRIVE_TOKEN_FILE` / `AGENT_CONTROL_GOOGLE_DRIVE_FILES` simultaneously.
4. Restrict the dedicated native Google identity: share only the intended folders as **Reader**; no ordinary admin account or domain-wide delegation. Account-wide unrelated files/shares can make verification fail. Folder inheritance and additional child grants remain the native owner’s responsibility.
5. Restart the server, log in as an Agent Control administrator and click **Connect Google account** in Applications. Google login/consent remains in Google’s own page. Offline access/consent permits server-side renewal; no access/refresh token is copied manually. Testing-mode grants can expire according to Google’s app rules; reconnect when required.
6. In Resources, select native reader folders from the bounded server list or paste their exact Drive folder links. Registration alone grants no agent access. Verify native access; unexpected visible objects or incomplete results block execution.
7. Enable the connector in Applications, configure tool permission and per-agent folder permission, and issue a scoped agent credential. Typical settings: search ALLOW, read ASK, and a deliberately small folder grant. Every absent tool/resource grant defaults DENY.

The server also requires its existing `AGENT_CONTROL_EXECUTION_ENABLED=true` for execution; emergency stop, connector state, agent state and current credential scopes remain authoritative. OAuth connection does not enable execution or confer a permission.

## Native permissions and resource contract

Google provides no read-only folder-ID-specific OAuth scope. This implementation requests **only `drive.readonly`** and refuses additional returned scopes/non-Bearer token types. `drive.file` was not selected: it permits file modification under the identity’s rights and does not supply the requested readonly folder-tree boundary. See [Google’s scope definitions](https://developers.google.com/workspace/drive/api/guides/api-specific-auth).

Native restriction comes from the dedicated identity’s Reader sharing. A bounded inventory of listed visible, non-trashed objects and selected folders’ non-editable capability is checked before each managed execution. This is a snapshot, not a certification of every effective ACL or every link-accessible file. Only a configured root and verified current ancestry may return data. A move outside that root fails its read. Native permission changes between calls remain a provider race; already admitted requests may finish after platform revocation.

Bounds: 20 active folders, 100 registered folders, inventory at most 1,000 objects / 10 pages, parent depth 20, 100 candidate folders with manual exact-link alternative. Exceeding or incompletely verifying the inventory fails closed. Native responses are bounded to 256 KiB and requests are time-bounded with no redirects, proxies or retries. Use a small dedicated pilot perimeter. Large enterprise accounts need a different inventory strategy before use.

Both tools require the Agent Control `resource_id` UUID. `google-drive.files.search` returns bounded direct-child metadata (up to 20 results), with optional `folder_id` for navigation inside that root. `google-drive.files.read` takes `file_id` and optional `max_chars` (maximum 6,000), reading only Docs/plain text. Native IDs and names returned by the provider are untrusted data. There are no write, sharing, deletion or arbitrary-URL tools.

Tool and resource policies combine with the stricter DENY > ASK > ALLOW rule. Discovery omits denied tools and exposes only currently granted enabled roots in schema enums/labels. Admission and dispatch recheck current root authority; exact approval binds the root and all validated parameters, identity, credential and idempotency key. Parent grants cannot bypass separately configured nested roots, including disabled ones: choose the child’s own resource permission. Parent search does not return another configured root as a traversal path.

Disabling a root invalidates verification and blocks its new platform requests. It does **not** remove Google ACLs; the readonly connection cannot change native sharing. Disconnect erases local credentials/pending OAuth attempts, disables the connector and attempts native grant revocation. Failed native revocation is reported; review it in the Google account. Resource/policy records remain for audit and do not automatically grant a new native connection.

The old exact-file token adapter remains only as an explicitly configured compatibility mode. It does not inherit managed-folder permissions or refresh, and cannot coexist with integrated OAuth. No automatic fallback broadens the new managed flow.

## OAuth and secret boundaries

S256 PKCE and random state use the maintained `oauth2` crate. The server persists only a keyed state reference and encrypted verifier; a ten-minute HttpOnly/Lax cookie binds the callback to the initiating browser, original current enabled administrator session and configured client. Normal human cookies remain SameSite Strict. Consumption is one-use and authority is rechecked after network exchange. Logout, session expiry, disabled administrators, wrong cookies/state/issuer, replay and callbacks after disconnect fail before storing a connection.

The client secret is loaded only from a private owned file. Native access/refresh tokens are AES-GCM encrypted with purpose separation and per-row associated data; the model and browser never receive them. Refresh is serialized; revoked grants or invalid scopes force reconnection and disable access. Generation/resource-revision checks prevent an old inventory response from revalidating a changed connection/perimeter. Token renewal and connection removal are recorded without secret payloads. Fixed Google endpoints cannot be supplied by agents.

Encryption is local storage protection, not a managed vault. A trusted host/database operator can access application key material. MFA, managed secret storage and independent review remain prerequisites for client deployment.

## Operator experience and audit

Operations uses actual current credentials and audit/execution rows. It shows completed actions/refused attempts over 24 hours, pending approvals, uncertain outcomes, connection interventions and the agent’s last observed request/completed action. Stateless MCP provides no reliable persistent “online” flag; active credentials and recent requests are separate observations.

Data consulted shows returned folder/document metadata for managed executions and distinguishes search metadata from bounded text reads. The audit ledger retains trusted root UUIDs and keyed object references; names/native IDs are encrypted in separate tables and decrypted for authorized operator views. No document text, OAuth codes/verifiers/tokens or raw provider exception is kept in audit. Metadata projections show the latest stored name; they do not reconstruct historical content. Phase 3 legacy events have no resource observations and are not fabricated.

Human-only administration routes cover connection status/start/callback/disconnect/verify/candidates, resource registration/state and resource grants. Agents cannot invoke these routes; reviewers can use their existing review surfaces but cannot administer connections/resources. Both MCP and REST execution continue through the common governance engine.

## Validation and review

Local validation: **143 default Rust tests passed**, **9 frontend checks passed**, Clippy with warnings denied and Rust formatting passed. The optional historical parity test passed with 28 matching HTTP observations and a separate Python MCP SDK workflow. The standalone optimized release builds and its help works with an empty PATH. Browser checks on an owned copy of Phase 3 pilot data confirmed Operations, the missing-client setup wizard, empty Resources and effective agent denial after credential revocation. Execution was disabled in that preview; no new live provider call was made. Ubuntu CI runs on the review branch; its final status is available on the pull request. Native Google fixture tests cover exchange/refresh, authority/PKCE binding, unexpected scopes/token types, invalid grants, revoked-session exchange, disconnect/pending callbacks, stale generation/revision, native broad/incomplete visibility, exact resource approval, policy denial, minimal audit and nested-root bypass attempts. HTTP tests exercise separate administration authentication, CSRF, reviewer restrictions and the real OAuth callback transport without live provider writes.

The implementer reviews trust boundaries, secret/metadata persistence, default denial, exact approvals, native request bounds and module responsibilities. This is not the requested future independent security review. No Google account/project/shares were created; no live integrated OAuth claim or client deployment is made.

Protected-source verification: ZNode’s 9 checkouts / 11,858 files match the existing baseline. Archived Python has no diff. Dovehouse’s previous baseline no longer matches: 9 website source files changed and a new `c9d8add` commit was observed during this work. Agent Control operations made no writes to Dovehouse; those changes were left untouched rather than reverted. The original baseline was not replaced.
