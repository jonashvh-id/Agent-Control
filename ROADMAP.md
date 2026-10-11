# Roadmap

Implemented foundation: standalone Rust executable, embedded existing frontend, human/agent identity boundaries, scoped credentials, DENY/ASK/ALLOW policies, encrypted exact human previews, one-time approvals, durable governed execution, explicit uncertainty, revocation/global stop, minimized audit, official Rust MCP Streamable HTTP and one GitHub issue connector. See the migration validation for actual tests and limits.

Next work remains separate: MCP OAuth, SSO/MFA, managed secret storage/rotation, tenant and fine-grained human permissions, provider reconciliation/cancellation, externally verifiable audit integrity, operational/load testing and independent security assessment. Additional connectors should follow the same governed interface after the first connector's architecture is accepted.

Migration does not deploy publicly, publish a release, migrate customer data automatically or add GitHub capabilities. Historical Phase 2 roadmap is retained in `docs/reviews/phase2/`.
