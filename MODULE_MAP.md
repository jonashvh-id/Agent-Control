# Module map

The primary runtime is Rust. See [RUST_MODULE_MAP.md](RUST_MODULE_MAP.md) for current responsibilities, public interfaces and dependency boundaries. The historical Python Phase 2 map is preserved in [docs/reviews/phase2/MODULE_MAP.md](docs/reviews/phase2/MODULE_MAP.md), and its source is archived under `reference/python/`.


The v0.5 OAuth/resource/operations extension preserves the four-crate Rust foundation. Its current contract, limitations and validation are recorded in [Phase 4](docs/PHASE4_OAUTH_RESOURCES.md); historical migration counts above describe their original checkpoint.
