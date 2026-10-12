import argparse
import getpass
import re
import sqlite3

import uvicorn

from backend.app.bootstrap import compose
from backend.app.core.config.settings import Settings
from backend.app.core.security.passwords import hash_password


def main():
    parser = argparse.ArgumentParser(description="Agent Control local foundation")
    parser.add_argument("command", choices=["serve", "create-admin"])
    parser.add_argument("--username", default="admin")
    parser.add_argument("--port", type=int, default=8010)
    args = parser.parse_args()
    if args.command == "serve":
        uvicorn.run("backend.app.main:app", host="127.0.0.1", port=args.port, proxy_headers=False)
        return
    username = args.username.strip().lower()
    if not re.fullmatch(r"[a-z0-9_.@-]{1,80}", username):
        parser.error("Invalid administrator username")
    password = getpass.getpass("New administrator password: ")
    if password != getpass.getpass("Repeat password: "):
        parser.error("Passwords differ")
    password_hash = hash_password(password)
    app = compose(Settings.from_env())
    try:
        with app.database.connect(write=True) as db:
            human_id = app.identity.repo.create(db, username, password_hash)
            app.audit.record(
                db, actor_kind="human", actor_id=human_id, action="administrator.bootstrap", decision="allow"
            )
    except sqlite3.IntegrityError:
        parser.error("Username already exists")
    print("Administrator created in the Agent Control data directory.")


if __name__ == "__main__":
    main()
