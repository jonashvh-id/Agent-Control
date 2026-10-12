CREATE TABLE connection_secrets (
 connector TEXT PRIMARY KEY, state TEXT NOT NULL CHECK(state IN ('disconnected','connected','reauth_required')),
 ciphertext TEXT NOT NULL DEFAULT '', generation INTEGER NOT NULL DEFAULT 0,
 verification TEXT NOT NULL DEFAULT '{}', updated_at TEXT NOT NULL, resource_revision INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE oauth_attempts (
 state_hash TEXT PRIMARY KEY, connector TEXT NOT NULL, human_id TEXT NOT NULL REFERENCES humans(id),
 session_hash TEXT NOT NULL, cookie_hash TEXT NOT NULL, verifier_ciphertext TEXT NOT NULL,
 expires_at REAL NOT NULL, consumed INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE resources (
 id TEXT PRIMARY KEY, connector TEXT NOT NULL, provider_hash TEXT NOT NULL,
 metadata_ciphertext TEXT NOT NULL, enabled INTEGER NOT NULL DEFAULT 1,
 created_by TEXT NOT NULL REFERENCES humans(id), created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
 UNIQUE(connector,provider_hash)
);
CREATE TABLE resource_policies (
 agent_id TEXT NOT NULL REFERENCES agents(id), resource_id TEXT NOT NULL REFERENCES resources(id),
 mode TEXT NOT NULL CHECK(mode IN ('deny','ask','allow')), updated_by TEXT NOT NULL REFERENCES humans(id),
 updated_at TEXT NOT NULL, PRIMARY KEY(agent_id,resource_id)
);
CREATE TABLE resource_objects (
 ref TEXT PRIMARY KEY, resource_id TEXT NOT NULL REFERENCES resources(id), metadata_ciphertext TEXT NOT NULL
);
ALTER TABLE audit_events ADD COLUMN resource_ref TEXT NOT NULL DEFAULT '';
ALTER TABLE audit_events ADD COLUMN object_refs TEXT NOT NULL DEFAULT '[]';
CREATE INDEX audit_agent_sequence ON audit_events(agent_id,sequence DESC);
CREATE INDEX oauth_attempt_expiry ON oauth_attempts(expires_at);
