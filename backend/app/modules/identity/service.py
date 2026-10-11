import secrets

from backend.app.core.errors.domain import DomainError
from backend.app.core.security.passwords import hash_password, verify_password
from backend.app.core.security.tokens import hash_token

from .repository import IdentityRepository


class IdentityService:
    def __init__(self, database, settings, audit):
        self.database, self.settings, self.audit = database, settings, audit
        self.repo = IdentityRepository()
        self.dummy_hash = hash_password("UnknownAccount9!" + secrets.token_hex(8))

    def login(self, username, password, ip):
        username = username.strip().lower()
        key = hash_token(username + "|" + ip, self.database.secret)
        token, csrf = secrets.token_urlsafe(32), secrets.token_urlsafe(32)
        with self.database.connect(write=True) as db:
            if not self.repo.attempt(db, key):
                error = DomainError("login_rate_limited", 429)
            else:
                human = self.repo.by_username(db, username)
                valid = verify_password(password, human["password_hash"] if human else self.dummy_hash)
                if not human or not valid or not human["enabled"]:
                    self.audit.record(
                        db, actor_kind="unknown", actor_id="", action="login", decision="denied"
                    )
                    error = DomainError("invalid_credentials", 401)
                else:
                    self.repo.issue_session(
                        db,
                        human["id"],
                        hash_token(token, self.database.secret),
                        csrf,
                        self.settings.session_seconds,
                    )
                    db.execute("DELETE FROM login_attempts WHERE key=?", (key,))
                    self.audit.record(
                        db, actor_kind="human", actor_id=human["id"], action="login", decision="allow"
                    )
                    return token, {
                        "id": human["id"],
                        "username": human["username"],
                        "role": human["role"],
                        "csrf": csrf,
                    }
        raise error

    def authenticate(self, token):
        with self.database.connect() as db:
            human = self.repo.session(db, hash_token(token or "", self.database.secret))
            if not human:
                raise DomainError("human_authentication_required", 401)
            return dict(human)

    def logout(self, token, actor):
        with self.database.connect(write=True) as db:
            self.repo.revoke_session(db, hash_token(token, self.database.secret))
            self.audit.record(db, actor_kind="human", actor_id=actor["id"], action="logout", decision="allow")
