"""Operator configuration: finite repository allowlist and fixed public GitHub origin."""

import re
from dataclasses import dataclass
from pathlib import Path

REPOSITORY = re.compile(r"^[A-Za-z0-9][A-Za-z0-9-]{0,38}/[A-Za-z0-9_.-]{1,100}$")
READ_ONLY_SOURCE = "jonashvh-id/zetako-node"
DEMO_REPOSITORY = "jonashvh-id/agent-control"


@dataclass(frozen=True)
class GitHubConfig:
    repositories: tuple[str, ...]
    token_file: Path | None = None
    credential_source: str = "file"
    timeout_seconds: float = 10

    def __post_init__(self):
        if not self.repositories or len(self.repositories) > 20:
            raise ValueError("Configure a finite GitHub repository allowlist")
        for repository in self.repositories:
            if not REPOSITORY.fullmatch(repository) or repository.split("/")[1] in {".", ".."}:
                raise ValueError("Invalid GitHub repository identifier")
            if repository.lower() == READ_ONLY_SOURCE:
                raise ValueError("ZNode repository must remain read-only and is excluded")
        if self.credential_source not in {"file", "gh-demo"}:
            raise ValueError("Invalid GitHub credential source")
        if self.credential_source == "file" and self.token_file is None:
            raise ValueError("GitHub requires a dedicated token file")
        if self.credential_source == "gh-demo" and tuple(r.lower() for r in self.repositories) != (
            DEMO_REPOSITORY,
        ):
            raise ValueError("gh-demo is restricted to jonashvh-id/Agent-Control")
        if not 0 < self.timeout_seconds <= 15:
            raise ValueError("GitHub requests require a bounded timeout")

    def permits(self, repository):
        return repository.lower() in {r.lower() for r in self.repositories}
