"""Read credentials only at dispatch. Nothing is copied into SQLite or configuration."""

import os
import stat
import subprocess

from pydantic import SecretStr

from backend.app.core.errors.domain import DomainError


class GitHubCredentialSource:
    def __init__(self, config):
        self.config = config

    def token(self):
        if self.config.credential_source == "gh-demo":
            try:
                command = subprocess.run(
                    ["gh", "auth", "token", "--hostname", "github.com"],
                    capture_output=True,
                    text=True,
                    timeout=5,
                    check=False,
                )
            except (OSError, subprocess.TimeoutExpired):
                raise DomainError("github_credential_unavailable", 503) from None
            if command.returncode:
                raise DomainError("github_credential_unavailable", 503)
            raw = command.stdout.strip()
        else:
            try:
                fd = os.open(self.config.token_file, os.O_RDONLY | os.O_NOFOLLOW)
                with os.fdopen(fd, "r") as stream:
                    info = os.fstat(stream.fileno())
                    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
                        raise DomainError("github_token_file_requires_private_permissions", 503)
                    raw = stream.read(4097).strip()
            except OSError:
                raise DomainError("github_credential_unavailable", 503) from None
        if not raw or len(raw) > 4096 or any(c.isspace() for c in raw):
            raise DomainError("github_credential_unavailable", 503)
        return SecretStr(raw)
