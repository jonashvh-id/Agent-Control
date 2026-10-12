# Migration validation

This document records the completed Rust migration checkpoint. New Phase 3 product/Google/Codex evidence and current test counts are tracked separately in [PHASE3_REPORT.md](docs/reviews/PHASE3_REPORT.md); the historical coverage and counts below are not measurements of the new adapter.

Local validation performed on macOS arm64 with Rust 1.89, against isolated owned data and providers. Ubuntu 24.04 / Rust 1.89 is exercised by [GitHub Actions](.github/workflows/ci.yml); its result is a merge gate for [PR #2](https://github.com/jonashvh-id/Agent-Control/pull/2). The primary application runtime uses Rust and embedded JavaScript assets. No live GitHub write, deployment, release publication or customer-data migration is part of these checks.

## Commands

```sh
cargo build --release --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
(cd frontend && npm run check)
```

Optional migration comparison, using the Python test environment retained from Phase 2:

```sh
cargo test -p agent-control-server --test python_parity -- --ignored --nocapture
```

Set `AGENT_CONTROL_PARITY_PYTHON` to another test interpreter if needed. Install the historical requirements into an isolated test environment from `reference/python/requirements.lock`. The tool restores the pinned Git commit into a temporary directory, runs its API with an isolated mocked GitHub transport, and compares it to an isolated Rust fixture server. It also launches a separate Python MCP SDK client against Rust. Python is only the test driver/reference here; default Rust tests and application startup do not require it.

The second opt-in test, `browser_fixture`, starts a temporary isolated server for manual browser QA and waits for Ctrl-C. It uses a test-only provider. Its default ignored status is intentional, not an unfinished runtime feature; both opt-in checks have been exercised separately.

Optional coverage:

```sh
cargo llvm-cov --workspace --ignore-filename-regex '/tests/|/adapter/tests.rs' --json --output-path /private/test-output/coverage.json
```

This uses cargo-llvm-cov 0.6.21 and llvm-tools-preview. At checkpoint `9f29a1a`, the measured production-file coverage was 89.88% of lines and 85.29% of regions; details are in `docs/rust/coverage-summary.json`. Stable Rust reports line/region coverage here; it is not the same metric as the historical combined Python statement/branch percentage. Coverage is evidence about exercised paths, not a security certification. Entry-point and optional demo/config paths have gaps; CLI/standalone/browser checks are separately recorded.

## Observed results

| Check | Result and evidence |
|---|---|
| Original Python baseline | 65 tests passed before changes; pinned reference preserved in history/archive |
| Rust automated checks | 112 passed, zero failures; two opt-in tests excluded from routine run; `docs/rust/test-results.json` |
| Cross-language comparison | One opt-in validation passed; 28 matching HTTP observations, plus separate-process Python SDK workflow |
| Native MCP client | Official Rust SDK connects over HTTP, discovers permitted tools, reads, receives ASK, consumes a human-approved exact request, receives a replay receipt and is blocked after revocation |
| Actual GitHub adapter | Native HTTP fixtures verify headers/path, safe DTOs, native denials, redirects, oversized/malformed results, real timeout uncertainty, private credential rotation/permissions and domain approval enforcement |
| Security branches | Default-deny, mandatory confirmation, scopes, exact binding, expiry/disabled approver, policy changes, revocations, concurrent claims, crash recovery, unknown outcomes, API/MCP boundaries, audit minimization and provider-instruction rejection; see SECURITY_PARITY.md |
| Standalone startup | Copied executable starts from unrelated directory with empty PATH, serves health/UI and enforces auth; no Python/workspace dependency |
| Release and CLI | Release build and interactive native admin creation succeeded; new Argon2 account login/logout and embedded v0.3 assets verified; `docs/rust/release-validation.json` |
| Frontend automated checks | 2 Node tests passed; existing interface retained |
| Browser workflows | Login, exact approval preview/decision, agent registration with empty optional fields, agent pause, policy save, audit/navigation and global stop worked on isolated Rust test data |
| Release UI | CLI-created QA account authenticated against the native release server; existing dashboard renders embedded v0.3 |
| Mobile | 390 × 844 viewport; document width 390, no overflow |
| Browser diagnostics | Only expected anonymous `/auth/me` 401 at startup; no JavaScript exceptions or later browser errors observed |
| Modularity | Three crates and small domain/repository/transport/provider modules; `docs/rust/modularity.json` |
| ZNode | 9 checkouts and 11,858 file hashes plus Git state unchanged; `docs/rust/znode-verification.json` |
| Dovehouse | 6 repositories and 2,763 file hashes plus Git state unchanged; `docs/rust/dovehouse-verification.json`; private baseline retained outside Git |

The behavioral transcript intentionally compares meaningful statuses/fields rather than random IDs, timestamps, independently keyed fingerprints or ciphertext bytes. Those bindings/crypto properties are separately tested in Rust. The original live issue #1 remains Phase 2 Python evidence; it is not presented as a new Rust provider write. Routine provider tests remain isolated.

## Known gaps and deployment limits

The API comparison is 28 scenarios, not exhaustive differential proof for every possible input. The Rust suite adds sensitive boundary cases but does not prove absence of bugs. Supply-chain advisory auditing, penetration/load testing, enterprise certification, OAuth MCP, SSO/MFA, managed vaults, tenancy, external audit integrity and reconciliation/cancellation are not completed by this migration.

Private-file guarantees currently require Unix; local macOS arm64 and the Ubuntu CI job provide platform checks. A TLS reverse proxy and dedicated repository-scoped native credential are needed for operational network use. Revocation prevents new admissions but cannot undo provider calls already admitted. No exactly-once provider claim is made.

Existing SQLite data is not automatically migrated; rollback and future migration require the separate plan in `docs/rust/DATA_MIGRATION.md`. Python remains historical reference, never a primary runtime dependency. The migration is prepared on a dedicated branch; fusion requires a successful Ubuntu job for the reviewed candidate revision. See [CODE_REVIEW.md](docs/rust/CODE_REVIEW.md) for corrections and review limits.


The v0.5 OAuth/resource/operations extension preserves the four-crate Rust foundation. Its current contract, limitations and validation are recorded in [Phase 4](docs/PHASE4_OAUTH_RESOURCES.md); historical migration counts above describe their original checkpoint.
