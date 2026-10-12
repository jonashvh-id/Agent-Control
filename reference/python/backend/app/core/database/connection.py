"""SQLite FK/WAL/busy-timeout settings adapted from zetako/sqlite.py."""

import os
import secrets
import sqlite3
from contextlib import contextmanager
from pathlib import Path

from .migrations import migrate


class Database:
    def __init__(self, data_dir: Path):
        self.path = data_dir / "agent-control.sqlite3"
        data_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
        if data_dir.is_symlink() or self.path.is_symlink():
            raise ValueError("Runtime paths must not be symlinks")
        marker = data_dir / ".agent-control"
        if not marker.exists():
            if any(data_dir.iterdir()):
                raise ValueError("Use an empty data directory owned by Agent Control")
            marker.write_text("agent-control-v1\n")
        if marker.read_text() != "agent-control-v1\n":
            raise ValueError("Invalid runtime ownership marker")
        os.chmod(data_dir, 0o700)
        key_path = data_dir / "token.key"
        if not key_path.exists():
            if self.path.exists():
                raise ValueError("Existing database is missing its credential key")
            fd = os.open(key_path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
            with os.fdopen(fd, "w") as stream:
                stream.write(secrets.token_urlsafe(48))
        if key_path.is_symlink():
            raise ValueError("Credential key must not be a symlink")
        os.chmod(key_path, 0o600)
        self.secret = key_path.read_text().strip()
        if len(self.secret) < 40:
            raise ValueError("Invalid credential key")
        with self.connect() as db:
            db.executescript(Path(__file__).with_name("schema.sql").read_text())
            migrate(db)
        os.chmod(self.path, 0o600)

    @contextmanager
    def connect(self, *, write=False):
        db = sqlite3.connect(self.path, timeout=5)
        db.row_factory = sqlite3.Row
        db.execute("PRAGMA foreign_keys = ON")
        db.execute("PRAGMA busy_timeout = 5000")
        db.execute("PRAGMA journal_mode = WAL")
        try:
            if write:
                db.execute("BEGIN IMMEDIATE")
            yield db
            db.commit()
        except BaseException:
            db.rollback()
            raise
        finally:
            db.close()
