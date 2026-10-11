# External adapters

One GitHub pilot adapter is implemented in `github/`. It is registered only with explicit operator configuration and initializes disabled. Native permissions and a finite repository allowlist constrain its two operations: issue listing and mandatory-approved issue creation. No repository deletion tool exists. ZNode is excluded.

Trusted adapter code implements `ConnectorAdapter.execute`; the GitHub adapter additionally supplies an encrypted human-preview projection, validates repository constraints and returns a minimized result receipt. Backend registration/configuration/lifecycle remain in backend modules. Both REST and MCP delegate to the shared gateway, not adapters directly.

New adapters require fixed/allowlisted destinations, least-privilege credentials, bounded I/O/results, no remote schemas/redirects/arbitrary proxy, meaningful exact human preview, replay/reconciliation design and end-to-end denied-path tests. Long-running cancellation, worker fencing and managed vault remain future work. Do not add more connectors as part of this pilot.
