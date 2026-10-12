# Historical Python reference

`python/` is the archived Phase 2 backend from commit `b946766e705856131b59f386b5ef657310f7dfdf`, retained for behavioral comparison. It is not part of the Rust application build or runtime. The original frontend is included here solely to keep this reference runnable. Do not deploy it as the new runtime or edit it to make parity tests pass.

The pinned Git commit is authoritative. Migration tooling restores it to an isolated temporary directory automatically. See the root MIGRATION_VALIDATION.md for the comparison command. Historical Python build/dependency metadata remains inside this directory; no Python backend or package metadata is required by the primary application.
