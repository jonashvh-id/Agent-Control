import os
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlsplit

from backend.app.modules.connectors.github_config import GitHubConfig


@dataclass(frozen=True)
class Settings:
    data_dir: Path
    origin: str = "http://127.0.0.1:8010"
    secure_cookies: bool = False
    execution_enabled: bool = False
    session_seconds: int = 28800
    approval_seconds: int = 900
    github: GitHubConfig | None = None
    mcp_enabled: bool = True

    def __post_init__(self):
        if (
            self.github
            and self.github.credential_source == "gh-demo"
            and urlsplit(self.origin).hostname not in {"127.0.0.1", "localhost", "::1"}
        ):
            raise ValueError("gh-demo credentials require a loopback origin")

    @classmethod
    def from_env(cls):
        origin = os.getenv("AGENT_CONTROL_ORIGIN", "http://127.0.0.1:8010").rstrip("/")
        secure = os.getenv("AGENT_CONTROL_SECURE_COOKIES", "false").lower() == "true"
        parsed = urlsplit(origin)
        if parsed.scheme not in {"http", "https"} or not parsed.hostname or parsed.path or parsed.username:
            raise ValueError("AGENT_CONTROL_ORIGIN must be an HTTP(S) origin")
        if parsed.scheme == "http" and parsed.hostname not in {"127.0.0.1", "localhost", "::1"}:
            raise ValueError("Non-local origins require HTTPS")
        if parsed.scheme == "https" and not secure:
            raise ValueError("HTTPS requires secure cookies")
        repositories = tuple(
            r.strip() for r in os.getenv("AGENT_CONTROL_GITHUB_REPOSITORIES", "").split(",") if r.strip()
        )
        source = os.getenv("AGENT_CONTROL_GITHUB_CREDENTIAL_SOURCE", "file")
        token_file = os.getenv("AGENT_CONTROL_GITHUB_TOKEN_FILE", "")
        github = (
            GitHubConfig(repositories, Path(token_file) if token_file else None, source)
            if repositories
            else None
        )
        return cls(
            Path(os.getenv("AGENT_CONTROL_DATA_DIR", "var")).absolute(),
            origin,
            secure,
            os.getenv("AGENT_CONTROL_EXECUTION_ENABLED", "false").lower() == "true",
            github=github,
            mcp_enabled=os.getenv("AGENT_CONTROL_MCP_ENABLED", "true").lower() == "true",
        )
