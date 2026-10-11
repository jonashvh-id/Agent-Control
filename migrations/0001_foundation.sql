CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY CHECK(version=1));
INSERT OR IGNORE INTO schema_version VALUES(1);
CREATE TABLE IF NOT EXISTS humans (
 id TEXT PRIMARY KEY, username TEXT UNIQUE NOT NULL, password_hash TEXT NOT NULL,
 role TEXT NOT NULL CHECK(role IN ('admin','reviewer')), enabled INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE IF NOT EXISTS sessions (
 token_hash TEXT PRIMARY KEY, human_id TEXT NOT NULL REFERENCES humans(id),
 csrf TEXT NOT NULL, expires_at REAL NOT NULL
);
CREATE TABLE IF NOT EXISTS login_attempts (key TEXT PRIMARY KEY, attempts INTEGER NOT NULL, reset_at REAL NOT NULL);
CREATE TABLE IF NOT EXISTS agents (
 id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL, provider TEXT NOT NULL,
 runtime TEXT NOT NULL, scopes TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'active'
 CHECK(status IN ('active','paused','disabled')), created_by TEXT NOT NULL REFERENCES humans(id),
 created_at TEXT NOT NULL, updated_at TEXT NOT NULL, last_used_at TEXT
);
CREATE TABLE IF NOT EXISTS credentials (
 id TEXT PRIMARY KEY, agent_id TEXT NOT NULL REFERENCES agents(id), token_hash TEXT UNIQUE NOT NULL,
 scopes TEXT NOT NULL, status TEXT NOT NULL CHECK(status IN ('active','revoked')),
 created_at TEXT NOT NULL, revoked_at TEXT
);
CREATE TABLE IF NOT EXISTS policies (
 agent_id TEXT NOT NULL REFERENCES agents(id), connector TEXT NOT NULL, tool TEXT NOT NULL,
 mode TEXT NOT NULL CHECK(mode IN ('deny','ask','allow')), updated_by TEXT NOT NULL REFERENCES humans(id),
 updated_at TEXT NOT NULL, PRIMARY KEY(agent_id,connector,tool)
);
CREATE TABLE IF NOT EXISTS connector_state (id TEXT PRIMARY KEY, enabled INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS emergency_state (id INTEGER PRIMARY KEY CHECK(id=1), stopped INTEGER NOT NULL DEFAULT 0);
INSERT OR IGNORE INTO emergency_state VALUES(1,0);
CREATE TABLE IF NOT EXISTS approvals (
 id TEXT PRIMARY KEY, agent_id TEXT NOT NULL REFERENCES agents(id), credential_id TEXT NOT NULL REFERENCES credentials(id),
 connector TEXT NOT NULL, tool TEXT NOT NULL, input_hash TEXT NOT NULL, idempotency_key TEXT NOT NULL,
 input_summary TEXT NOT NULL, status TEXT NOT NULL CHECK(status IN ('pending','approved','denied','expired','consumed')),
 expires_at REAL NOT NULL, created_at TEXT NOT NULL, decided_by TEXT REFERENCES humans(id), decided_at TEXT
);
CREATE TRIGGER IF NOT EXISTS approval_immutable BEFORE UPDATE OF agent_id,credential_id,connector,tool,input_hash,
 idempotency_key,input_summary,expires_at,created_at ON approvals
 BEGIN SELECT RAISE(ABORT,'approval binding is immutable'); END;
CREATE TABLE IF NOT EXISTS executions (
 id TEXT PRIMARY KEY, agent_id TEXT NOT NULL REFERENCES agents(id), idempotency_key TEXT NOT NULL,
 request_hash TEXT NOT NULL, approval_id TEXT REFERENCES approvals(id),
 status TEXT NOT NULL CHECK(status IN ('prepared','dispatched','executed','failed','uncertain')), created_at TEXT NOT NULL,
 UNIQUE(agent_id,idempotency_key)
);
CREATE TABLE IF NOT EXISTS audit_events (
 sequence INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT UNIQUE NOT NULL, created_at TEXT NOT NULL,
 actor_kind TEXT NOT NULL, actor_id TEXT NOT NULL, human_requester_id TEXT NOT NULL DEFAULT '',
 agent_id TEXT NOT NULL DEFAULT '', connector TEXT NOT NULL DEFAULT '', tool TEXT NOT NULL DEFAULT '',
 action TEXT NOT NULL, decision TEXT NOT NULL, approval_id TEXT NOT NULL DEFAULT '',
 execution_id TEXT NOT NULL DEFAULT '', correlation_id TEXT NOT NULL, input_hash TEXT NOT NULL DEFAULT '',
 input_summary TEXT NOT NULL DEFAULT '{}'
);
CREATE TRIGGER IF NOT EXISTS audit_no_update BEFORE UPDATE ON audit_events
 BEGIN SELECT RAISE(ABORT,'audit is append-only'); END;
CREATE TRIGGER IF NOT EXISTS audit_no_delete BEFORE DELETE ON audit_events
 BEGIN SELECT RAISE(ABORT,'audit is append-only'); END;
CREATE INDEX IF NOT EXISTS approval_status ON approvals(status,expires_at);
CREATE INDEX IF NOT EXISTS audit_created ON audit_events(created_at);

ALTER TABLE approvals ADD COLUMN preview_ciphertext TEXT NOT NULL DEFAULT '';
ALTER TABLE executions ADD COLUMN dispatch_started_at TEXT;
ALTER TABLE executions ADD COLUMN result_receipt TEXT;
ALTER TABLE audit_events ADD COLUMN result_summary TEXT NOT NULL DEFAULT '{}';
CREATE TRIGGER approval_preview_immutable BEFORE UPDATE OF preview_ciphertext ON approvals BEGIN SELECT RAISE(ABORT,'approval preview is immutable'); END;
