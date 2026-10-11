# Validation results — October 11, 2026 (Asia/Makassar)

This is the historical phase 1 validation. Current phase 2 results (65 tests, MCP/GitHub and live issue evidence) are in [PHASE2_REPORT.md](../reviews/PHASE2_REPORT.md).

Completed against the standalone Agent Control project. ZNode application code, tests and databases were never executed.

| Check | Actual result |
|---|---|
| Backend/integration tests | 33 passed, no warnings or failures |
| Frontend Node tests | 2 passed |
| JavaScript syntax | All 15 UI modules parsed successfully |
| Ruff lint / formatting | Passed; 55 Python files formatted at final check |
| Installed dependencies | `pip check`: no broken requirements |
| Independent application start | Health and frontend assets passed from a temporary cwd |
| Packaged application | Final wheel installed into a fresh separate virtual environment; health/UI passed without workspace or ZNode imports |
| Wheel contents | UI/SQL bundled; source tests, environment files, credentials and databases excluded |
| Module imports | AST scan found no runtime imports from ZNode, zetako or its app package |
| Browser checks | Login, registration with optional fields empty, policy save, navigation to all requested pages, and emergency stop worked against isolated temporary data |
| Browser console | No errors/warnings after the completed UI checks |
| Desktop | Visually checked at 1440 × 1050 |
| Mobile | Visually checked at 390 × 844; document width 390, no page overflow |
| Modularity | Largest backend application module is 232 lines; no monolithic inherited service |
| ZNode preservation | 9 checkouts, 11,858 source-file hashes, HEADs, refs and Git status unchanged |

The initial browser check caught optional Boolean form attributes incorrectly set as strings, and mobile CSS ordering overriding responsive rules. Both were corrected and the affected flows were rechecked. Initial test assertion/path errors were corrected; the completed baseline has no failing tests. A legacy HTTP client deprecation was eliminated by using Starlette's current httpx2 test dependency.

The final wheel digest and package smoke results are in `package-validation.json`. Source preservation evidence is in `../migration/znode-before.json` and `../migration/znode-verification.json`. Copied code provenance is in `../migration/source-manifest.json` and `../migration/extracted-components.json`.

Browser review used an explicitly temporary account/data directory and test-only administration activity. Screenshots under `output/playwright/` show that temporary review instance, not production or seeded product activity. No real external integration executed. The temporary browser/server are closed after validation. No real account was provisioned in the default runtime, and no public deployment or production infrastructure change occurred.

Limits: source live-production provenance remains unconfirmed; the pinned source is current refactor, not a verified running-server snapshot. No full ZNode test suite, external provider integration, load/penetration test, supply-chain advisory audit or production security certification was performed. See `SECURITY_BOUNDARIES.md` and `ROADMAP.md` for the unimplemented security/product work.
