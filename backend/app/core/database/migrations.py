"""Additive versioned migrations for Agent Control only. No ZNode schemas are accepted."""


def migrate(connection):
    connection.execute("CREATE TABLE IF NOT EXISTS migrations(version INTEGER PRIMARY KEY)")
    if connection.execute("SELECT 1 FROM migrations WHERE version=2").fetchone():
        return
    additions = {
        "approvals": {"preview_ciphertext": "TEXT NOT NULL DEFAULT ''"},
        "executions": {"dispatch_started_at": "TEXT", "result_receipt": "TEXT"},
        "audit_events": {"result_summary": "TEXT NOT NULL DEFAULT '{}'"},
    }
    for table, columns in additions.items():
        existing = {row["name"] for row in connection.execute(f"PRAGMA table_info({table})")}
        for name, definition in columns.items():
            if name not in existing:
                connection.execute(f"ALTER TABLE {table} ADD COLUMN {name} {definition}")
    connection.execute("""CREATE TRIGGER IF NOT EXISTS approval_preview_immutable
                       BEFORE UPDATE OF preview_ciphertext ON approvals
                       BEGIN SELECT RAISE(ABORT,'approval preview is immutable'); END;""")
    connection.execute("INSERT INTO migrations VALUES(2)")
