# Rust migration code review — 2026-10-12

Scope: migration from pinned Python commit `b946766` through Rust checkpoint `9f29a1a` and final review corrections. This is an implementer review, not an independent security audit. Merge requires passing local checks and the Ubuntu CI job for the candidate revision.

## Findings corrected

| Finding | Reproduction before correction | Correction and regression evidence |
|---|---|---|
| REST Host checks discarded the port and parsed Host as a URL, accepting other ports and URL components on the configured hostname. MCP already required the configured authority. | `host_authority_rejected_on_rest` failed: `Host: 127.0.0.1` returned 200 despite the configured ephemeral port. | All routes require the exact configured Host authority. The test rejects missing/wrong ports, userinfo, paths and queries on health, authenticated administration and agent routes without provider calls. |
| GitHub read approvals omitted state, limit and page in their human preview. The fingerprint bound them, but reviewers could not see the complete operation. | `approved_read_preview_includes_filters_and_rejects_changes` failed: the encrypted human preview had no `state`. | Read previews show effective filter/pagination, including defaults. The real adapter test checks the decrypted human preview, rejects changed pagination after approval, and executes only the original parameters against a local provider. |

Native adapter response tests and governed adapter tests have separate files. Generated interface screenshots are ignored rather than included in source control.

## Reviewed boundaries

- Three-crate dependency direction and separation of HTTP/MCP transport, policy, approvals, durable ledger, audit and provider code.
- Current human/agent authentication, role enforcement, CSRF, Origin/Host, credential scope intersection and revocation.
- Validated exact request fingerprints, encrypted immutable previews, one-time consumption, expiry and current approver checks before dispatch.
- Atomic claims and durable dispatch before network I/O, concurrent duplicates, final authorization, recovery and explicit uncertainty with no blind retry.
- Fixed GitHub origin/repositories, native denial, bounded I/O, safe receipts, private credential files and sanitized errors/audit.
- Archived Python provenance, existing frontend contract, standalone startup, docs and exclusion of credentials/runtime files.

No unresolved blocking finding was identified in these reviewed paths after the corrections. This does not prove the absence of vulnerabilities. Data migration, tenancy, SSO/MFA, vaults, cancellation/reconciliation, external audit integrity and production/load/security certification remain outside this migration's acceptance criteria.

## Validation and merge gate

Local macOS arm64 validation: 112 Rust tests passed; 2 explicit opt-in tests are excluded from the default run. Clippy with warnings denied, rustfmt, 2 frontend tests and release build pass. Optional pinned Python/Rust validation passes 28 HTTP observations plus the separate Python MCP SDK workflow. no live provider write was performed.

[Ubuntu workflow](../../.github/workflows/ci.yml) runs on pull requests and pushes to main: Rust 1.89, formatting, clippy, all default tests (including isolated standalone startup), frontend checks, release build and release help with empty PATH. Actions are pinned to verified upstream commit hashes; job permissions are read-only and no application secret is supplied. Branch pushes are covered by the PR synchronize event, avoiding duplicate builds for a single commit. [PR #2](https://github.com/jonashvh-id/Agent-Control/pull/2) contains the authoritative CI result for each revision. Fusion is authorized only after its Ubuntu job succeeds and the PR remains mergeable.
