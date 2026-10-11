# Development rules

One file has one clear responsibility. Most application files should stay under approximately 300 lines. At 400–500 lines, review cohesion before adding more logic. Splitting files by arbitrary line count is not a substitute for separating responsibilities. Avoid giant service files, broad utility modules and inheritance hierarchies.

Composition belongs in `bootstrap.py`. Dependency direction is API → services → repositories / pure helpers → database. Repositories receive an explicit transaction when coordinated state changes must be atomic. Cross-module orchestration belongs in a focused application service such as the gateway; do not duplicate policy or authorization rules across routes or adapters.

Use strict Pydantic input models with unexpected properties rejected. Use JSON schema for trusted tool input/output contracts. Return sanitized domain error codes; never echo invalid credentials or raw adapter exceptions. Public health reveals only service health and release stage. Human endpoints require session authentication; mutations require CSRF. Agent gateway endpoints accept only scoped agent bearer credentials.

The policy engine is deterministic, scope-aware and default-deny. No agent gets human admin roles, creator privileges or hidden superuser scope. Adding a connector or a tool must not make it automatically discoverable or executable. Use the existing admission and dispatch checks for every transport.

Register trusted adapter code explicitly. Never load Python modules, shell commands, URLs or credentials supplied by an agent. Network adapters require documented destination allowlists, SSRF protections, bounded I/O, result validation and secret-vault access before registration. Do not enable blind retries for uncertain state-changing operations.

Approval payloads are immutable. Verify credential ownership, request fingerprint, expiry, decision and current authorizations. Consume grants with transactional state checks. Preserve the durable execution claim before external I/O. A failed/uncertain claim needs reconciliation; do not reset it as a shortcut to retry.

Tests use fresh temporary data directories and test-only adapters. Security changes require integration checks of blocked paths as well as allowed paths, concurrency and restart behavior. Do not run ZNode tests or migrations in its checkout. Do not make tests that merely repeat implementation logic or seed fake operational events in the product UI.

No secrets in fixtures outside explicit dummy test credentials, in logs, errors, browser storage, source or audit values. Dependency locks must not contain editable paths to another project. Package frontend assets from their single source tree; verify an installed wheel from outside the workspace.

Update the module map, extraction provenance, security boundaries and roadmap when behavior changes. Do not report an integration as working merely because its interface exists. Do not publicly deploy the foundation.
