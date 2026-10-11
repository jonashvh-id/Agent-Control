import pytest
from fastapi.testclient import TestClient

from backend.app.core.config.settings import Settings
from backend.app.core.database.connection import Database
from backend.app.main import create_app


def test_empty_standalone_runtime(tmp_path):
    with TestClient(create_app(Settings(tmp_path / "data")), base_url="http://127.0.0.1:8010") as client:
        assert client.get("/health").json() == {
            "status": "ok",
            "service": "agent-control",
            "stage": "foundation",
        }
        assert client.get("/").status_code == 200
        assert client.get("/ui/src/app.js").status_code == 200
        assert client.get("/api/v1/agents").status_code == 401
        assert client.get("/docs").status_code == 404


def test_runtime_refuses_existing_unowned_directory(tmp_path):
    (tmp_path / "customer.db").write_text("must not modify")
    with pytest.raises(ValueError, match="empty data directory"):
        Database(tmp_path)
    assert (tmp_path / "customer.db").read_text() == "must not modify"


def test_remote_schema_references_rejected():
    from backend.app.modules.connectors.registry import validate_local_schema

    with pytest.raises(ValueError, match="Remote schema"):
        validate_local_schema({"$ref": "https://example.com/untrusted.json"})


def test_parameters_reject_complexity_and_non_finite_json():
    from pydantic import ValidationError

    from backend.app.modules.gateway.models import ExecutionProposal

    base = dict(connector="test", tool="records.create", idempotency_key="client-key")
    value = {}
    for _ in range(34):
        value = {"nested": value}
    with pytest.raises(ValidationError):
        ExecutionProposal(**base, parameters=value)
    with pytest.raises(ValidationError):
        ExecutionProposal(**base, parameters={"x": float("nan")})


def test_phase1_owned_database_is_migrated_without_erasing_existing_records(tmp_path):
    import sqlite3
    from pathlib import Path

    directory = tmp_path / "legacy"
    directory.mkdir()
    (directory / ".agent-control").write_text("agent-control-v1\n")
    (directory / "token.key").write_text("legacy-credential-key-" + "a" * 40)
    database = directory / "agent-control.sqlite3"
    with sqlite3.connect(database) as db:
        db.executescript(Path("backend/app/core/database/schema.sql").read_text())
        db.execute("INSERT INTO humans VALUES('legacy-human','existing-admin','test-hash','admin',1)")
    migrated = Database(directory)
    with migrated.connect() as db:
        assert db.execute("SELECT username FROM humans").fetchone()[0] == "existing-admin"
        assert db.execute("SELECT version FROM migrations").fetchone()[0] == 2
        assert "preview_ciphertext" in {r["name"] for r in db.execute("PRAGMA table_info(approvals)")}
        assert "dispatch_started_at" in {r["name"] for r in db.execute("PRAGMA table_info(executions)")}
    Database(directory)  # Repeat startup must not rerun/erase the migration.


def test_approval_preview_cipher_rejects_modified_ciphertext():
    from backend.app.core.errors.domain import DomainError
    from backend.app.core.security.sealed import PreviewCipher

    cipher = PreviewCipher("independent-fixture-key")
    value = cipher.seal({"body": "private approval body"})
    assert "private approval body" not in value
    assert cipher.open(value) == {"body": "private approval body"}
    with pytest.raises(DomainError, match="preview_unavailable"):
        cipher.open(value[:-12] + "modified")


def test_operator_config_registers_one_connector_disabled_until_explicit_enable(tmp_path):
    from backend.app.bootstrap import compose
    from backend.app.modules.connectors.github_config import GitHubConfig

    token = tmp_path / "github.token"
    token.write_text("private-fixture-token")
    token.chmod(0o600)
    settings = Settings(
        tmp_path / "runtime", github=GitHubConfig(("demo/repo",), token), execution_enabled=True
    )
    services = compose(settings)
    assert len(services.registry.all()) == 2
    assert {tool.connector for tool in services.registry.all()} == {"github"}
    with services.database.connect() as db:
        assert not db.execute("SELECT enabled FROM connector_state WHERE id='github'").fetchone()[0]
    assert services.registry.get("github", "repos.delete") is None


def test_gh_demo_cannot_be_enabled_on_nonlocal_origin(tmp_path):
    from backend.app.modules.connectors.github_config import GitHubConfig

    config = GitHubConfig(("jonashvh-id/Agent-Control",), credential_source="gh-demo")
    with pytest.raises(ValueError, match="loopback"):
        Settings(tmp_path / "data", origin="https://public.example", secure_cookies=True, github=config)
