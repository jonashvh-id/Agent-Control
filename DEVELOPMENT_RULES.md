# Development rules

Use the Rust architecture and security requirements in [AGENTS.md](AGENTS.md). Keep changes scoped, modular and reviewable. Follow the dependency boundaries in [RUST_MODULE_MAP.md](RUST_MODULE_MAP.md) and run the checks in [MIGRATION_VALIDATION.md](MIGRATION_VALIDATION.md).

Security and functional parity take precedence over speed. A passing compilation or a high coverage number alone does not prove parity. Maintain documented behavioral contracts using isolated data/providers. Never use customer runtimes or unrelated projects as test environments.
