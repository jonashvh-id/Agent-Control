# Future data migration and rollback strategy

This implementation only initializes new owned Rust runtimes. The Rust ownership marker differs from Python's marker; old/customer directories are rejected before migration. SQLx migration 0001 is a fresh schema, not a converter for existing Python data. Never relabel an old directory to make it pass the guard.

A future migration needs an explicit maintenance plan and authorization for that dataset. Stop admissions, let known in-flight actions finish or reconcile them, and preserve all unknown outcomes without re-dispatch. Back up the original database, WAL-consistent snapshot, key and original application version in private operator storage. Work only on offline copies and maintain identifier/audit ordering and foreign-key relationships.

Passwords, token HMACs and preview ciphertext use different formats/keys. A converter must explicitly handle password rehash/reset, session/agent credential rotation, authenticated preview conversion and audit continuity. It must not import old active credentials blindly or replay historical uncertain writes. Dry-run the transformation and behavioral checks against separate new storage before a cutover is authorized.

Rollback keeps the original executable/data/key unchanged. Once new provider actions have occurred, reconcile external effects first and preserve the Rust ledger/audit; simply restoring a stale database could authorize duplicate writes. No automatic production migration, destructive conversion or live rollback script is included in this phase.
