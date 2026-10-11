import json
from dataclasses import replace

import httpx2
import pytest
from pydantic import SecretStr

from backend.app.core.errors.domain import DomainError
from backend.app.modules.connectors.github_config import GitHubConfig
from connectors.github.adapter import GitHubAdapter
from connectors.github.credentials import GitHubCredentialSource


@pytest.fixture
def config(tmp_path):
    token = tmp_path / "github.token"
    token.write_text("fixture-private-token")
    token.chmod(0o600)
    return GitHubConfig(("demo/repo",), token)


def issue(number=1):
    return {
        "number": number,
        "title": "Fixture issue",
        "state": "open",
        "html_url": f"https://github.com/demo/repo/issues/{number}",
    }


def test_real_adapter_uses_fixed_origin_native_credentials_and_minimized_result(config):
    calls = []

    def github(request):
        calls.append(request)
        assert request.url.scheme == "https" and request.url.host == "api.github.com"
        assert request.headers["authorization"] == "Bearer fixture-private-token"
        assert request.headers["x-github-api-version"] == "2026-03-10"
        assert request.url.path == "/repos/demo/repo/issues"
        if request.method == "GET":
            assert request.url.params["per_page"] == "2"
            return httpx2.Response(200, json=[issue(), {**issue(2), "pull_request": {}}])
        assert json.loads(request.content) == {"title": "Exact title", "body": "Exact body"}
        return httpx2.Response(
            201, json={**issue(), "body": "must-not-persist", "user": {"token": "must-not-persist"}}
        )

    adapter = GitHubAdapter(config, transport=httpx2.MockTransport(github))
    assert (
        len(
            adapter.execute("issues.list", {"repository": "demo/repo", "limit": 2}, idempotency_key="key")[
                "issues"
            ]
        )
        == 1
    )
    result = adapter.execute(
        "issues.create",
        {"repository": "demo/repo", "title": "Exact title", "body": "Exact body"},
        idempotency_key="key",
    )
    assert set(result["issue"]) == {"number", "title", "url", "state"}
    assert adapter.receipt(result)["issue_number"] == 1
    assert len(calls) == 2


@pytest.mark.parametrize(
    "status,code",
    [
        (401, "github_access_denied"),
        (403, "github_access_denied"),
        (404, "github_access_denied"),
        (429, "github_rate_limited"),
        (302, "github_request_failed"),
        (500, "github_request_failed"),
    ],
)
def test_native_permissions_errors_and_redirects_are_never_bypassed_or_retried(config, status, code):
    calls = []

    def denied(request):
        calls.append(request)
        return httpx2.Response(
            status, headers={"Location": "https://evil.example"}, json={"message": "token=secret-error"}
        )

    adapter = GitHubAdapter(config, transport=httpx2.MockTransport(denied))
    with pytest.raises(DomainError, match=code):
        adapter.execute(
            "issues.create", {"repository": "demo/repo", "title": "T", "body": ""}, idempotency_key="key"
        )
    assert len(calls) == 1


@pytest.mark.parametrize(
    "repository",
    ["../escape", "demo/repo/../../admin", "jonashvh-id/zetako-node", "other/repo", "https://evil.test"],
)
def test_repository_and_arbitrary_url_changes_never_reach_network(config, repository):
    adapter = GitHubAdapter(config, transport=httpx2.MockTransport(lambda _: pytest.fail("Unexpected I/O")))
    with pytest.raises(DomainError, match="repository_denied"):
        adapter.execute(
            "issues.create", {"repository": repository, "title": "T", "body": ""}, idempotency_key="key"
        )
    with pytest.raises(DomainError, match="tool_not_supported"):
        adapter.execute("repos.delete", {"repository": "demo/repo"}, idempotency_key="key")


def test_znode_and_gh_demo_restrictions_are_configuration_errors(config):
    with pytest.raises(ValueError, match="ZNode"):
        replace(config, repositories=("jonashvh-id/zetako-node",))
    with pytest.raises(ValueError, match="restricted"):
        replace(config, credential_source="gh-demo")
    with pytest.raises(ValueError, match="identifier"):
        replace(config, repositories=("demo/..",))


def test_secret_file_is_private_not_cached_and_symlinks_rejected(config, tmp_path):
    source = GitHubCredentialSource(config)
    assert isinstance(source.token(), SecretStr)
    config.token_file.write_text("rotated-credential")
    assert source.token().get_secret_value() == "rotated-credential"
    config.token_file.chmod(0o644)
    with pytest.raises(DomainError, match="private_permissions"):
        source.token()
    link = tmp_path / "link"
    link.symlink_to(config.token_file)
    with pytest.raises(DomainError, match="unavailable"):
        GitHubCredentialSource(replace(config, token_file=link)).token()


def test_provider_timeout_and_invalid_data_are_sanitized(config):
    def timeout(request):
        raise httpx2.ReadTimeout("token=secret", request=request)

    adapter = GitHubAdapter(config, transport=httpx2.MockTransport(timeout))
    with pytest.raises(DomainError, match="github_timeout"):
        adapter.execute("issues.list", {"repository": "demo/repo"}, idempotency_key="key")
    adapter = GitHubAdapter(
        config, transport=httpx2.MockTransport(lambda _: httpx2.Response(200, text="secret-non-json"))
    )
    with pytest.raises(DomainError, match="response_invalid"):
        adapter.execute("issues.list", {"repository": "demo/repo"}, idempotency_key="key")


def test_provider_result_url_and_size_are_validated(config):
    adapter = GitHubAdapter(
        config,
        transport=httpx2.MockTransport(
            lambda _: httpx2.Response(201, json={**issue(), "html_url": "https://evil.example"})
        ),
    )
    with pytest.raises(DomainError, match="response_invalid"):
        adapter.execute(
            "issues.create", {"repository": "demo/repo", "title": "T", "body": ""}, idempotency_key="key"
        )
    for unsafe_url in [
        "https://github.com/demo/repo/issues/1?token=private",
        "https://github.com/demo/repo/issues/1#private",
    ]:
        adapter = GitHubAdapter(
            config,
            transport=httpx2.MockTransport(
                lambda _, url=unsafe_url: httpx2.Response(201, json={**issue(), "html_url": url})
            ),
        )
        with pytest.raises(DomainError, match="response_invalid"):
            adapter.execute(
                "issues.create", {"repository": "demo/repo", "title": "T", "body": ""}, idempotency_key="key"
            )
    adapter = GitHubAdapter(
        config, transport=httpx2.MockTransport(lambda _: httpx2.Response(200, content=b"x" * 262145))
    )
    with pytest.raises(DomainError, match="result_too_large"):
        adapter.execute("issues.list", {"repository": "demo/repo"}, idempotency_key="key")


def test_gh_demo_reads_one_fixed_credential_without_persisting_or_exposing_errors(monkeypatch):
    import subprocess

    config = GitHubConfig(("jonashvh-id/Agent-Control",), credential_source="gh-demo")
    calls = []

    def run(args, **kwargs):
        calls.append(args)
        assert kwargs["capture_output"] and kwargs["timeout"] == 5
        return subprocess.CompletedProcess(args, 0, stdout="fixture-gh-private-token\n", stderr="")

    monkeypatch.setattr(subprocess, "run", run)
    token = GitHubCredentialSource(config).token()
    assert token.get_secret_value() == "fixture-gh-private-token"
    assert "fixture-gh-private-token" not in repr(token)
    assert calls == [["gh", "auth", "token", "--hostname", "github.com"]]
    monkeypatch.setattr(
        subprocess,
        "run",
        lambda *a, **kw: subprocess.CompletedProcess(a, 1, stdout="", stderr="secret-native-error"),
    )
    with pytest.raises(DomainError, match="github_credential_unavailable"):
        GitHubCredentialSource(config).token()

    def timeout(*a, **kw):
        raise subprocess.TimeoutExpired("private-error", 5)

    monkeypatch.setattr(subprocess, "run", timeout)
    with pytest.raises(DomainError, match="github_credential_unavailable"):
        GitHubCredentialSource(config).token()
