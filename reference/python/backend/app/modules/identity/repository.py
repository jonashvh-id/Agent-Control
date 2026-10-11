import time
import uuid


class IdentityRepository:
    def create(self, db, username, password_hash, role="admin"):
        human_id = uuid.uuid4().hex
        db.execute(
            "INSERT INTO humans(id,username,password_hash,role) VALUES(?,?,?,?)",
            (human_id, username, password_hash, role),
        )
        return human_id

    def by_username(self, db, username):
        return db.execute("SELECT * FROM humans WHERE username=?", (username,)).fetchone()

    def session(self, db, token_hash):
        return db.execute(
            """SELECT h.id,h.username,h.role,s.csrf FROM sessions s JOIN humans h
                          ON h.id=s.human_id WHERE s.token_hash=? AND s.expires_at>? AND h.enabled=1""",
            (token_hash, time.time()),
        ).fetchone()

    def issue_session(self, db, human_id, token_hash, csrf, seconds):
        db.execute("DELETE FROM sessions WHERE expires_at<=?", (time.time(),))
        db.execute(
            "INSERT INTO sessions VALUES(?,?,?,?)", (token_hash, human_id, csrf, time.time() + seconds)
        )

    def revoke_session(self, db, token_hash):
        db.execute("DELETE FROM sessions WHERE token_hash=?", (token_hash,))

    def attempt(self, db, key):
        now = time.time()
        db.execute("DELETE FROM login_attempts WHERE reset_at<=?", (now,))
        row = db.execute("SELECT attempts FROM login_attempts WHERE key=?", (key,)).fetchone()
        if row and row["attempts"] >= 5:
            return False
        db.execute(
            """INSERT INTO login_attempts VALUES(?,1,?) ON CONFLICT(key)
                   DO UPDATE SET attempts=attempts+1""",
            (key, now + 300),
        )
        return True
