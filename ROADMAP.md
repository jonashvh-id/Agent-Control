# Roadmap

## Retained and validated foundation

Selective password/tool/token/DOM reuse; standalone human sessions and agents; deterministic scope-first deny/ask/allow; approvals and lifecycle; append-only minimized audit; existing administration shell. See EXTRACTION_REPORT for the distinction between copy, adaptation and rewrite.

## Implemented GitHub/MCP pilot

- Official SDK Streamable HTTP transport, current bearer checks and per-agent discovery.
- Shared REST/MCP authorization/admission/dispatch, no API shortcut.
- Real GitHub issue listing and mandatory-approved issue creation in a finite allowlist.
- Native permissions, fixed TLS origin, bounded results, no redirects/proxies/automatic writes retries.
- Encrypted immutable exact previews and one-time credential-bound approval.
- Durable dispatch marker, minimized provider receipts and responsive revocation while I/O is in progress.
- SDK/HTTP end-to-end security regressions and original foundation checks.
- Minimal real GitHub proof is reported separately from the isolated provider tests in docs/reviews.

## Next operational work

Dedicated fine-grained or GitHub App credentials; OAuth MCP interoperability; managed vault/rotation; SSO/MFA and administrator lifecycle; credential expiry; resource policies per agent and requester delegation; worker fencing/leases/cancellation and uncertain-write reconciliation; tamper-evident external audit delivery/export/retention; backup/restore, rate limits, dependency advisory review, load and penetration testing.

Further connectors remain deferred until this first path is validated. No Google/Microsoft adapter, enterprise certification or public production deployment is claimed.
