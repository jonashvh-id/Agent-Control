"""Real GitHub REST adapter. Fixed origin, bounded reads, no redirects or automatic retries."""

import time
from urllib.parse import quote, urlsplit

import httpx2

from backend.app.core.errors.domain import DomainError

from .credentials import GitHubCredentialSource

API_ORIGIN = "https://api.github.com"
API_VERSION = "2026-03-10"


class GitHubAdapter:
    def __init__(self, config, credential_source=None, transport=None):
        self.config = config
        self.credentials = credential_source or GitHubCredentialSource(config)
        # Transport injection is for isolated tests, never operator/agent configuration.
        self.transport = transport

    def preview(self, tool, parameters):
        self.validate(tool, parameters)
        return {
            "repository": parameters["repository"],
            "title": parameters.get("title", ""),
            "body": parameters.get("body", ""),
            "operation": tool,
        }

    def validate(self, tool, parameters):
        if tool not in {"issues.list", "issues.create"}:
            raise DomainError("github_tool_not_supported", 403)
        if not self.config.permits(parameters.get("repository", "")):
            raise DomainError("github_repository_denied", 403)

    def execute(self, tool, parameters, *, idempotency_key):
        self.validate(tool, parameters)
        repository = parameters["repository"]
        owner, name = repository.split("/")
        endpoint = f"/repos/{quote(owner, safe='')}/{quote(name, safe='')}/issues"
        is_create = tool == "issues.create"
        kwargs = (
            {"json": {"title": parameters["title"], "body": parameters["body"]}}
            if is_create
            else {
                "params": {
                    "state": parameters.get("state", "open"),
                    "per_page": parameters.get("limit", 10),
                    "page": parameters.get("page", 1),
                }
            }
        )
        token = self.credentials.token()
        headers = {
            "Authorization": "Bearer " + token.get_secret_value(),
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": API_VERSION,
            "User-Agent": "Agent-Control/0.2",
        }
        # GitHub issue creation has no provider idempotency guarantee. Gateway claims prevent retries.
        deadline = time.monotonic() + self.config.timeout_seconds
        try:
            with httpx2.Client(
                base_url=API_ORIGIN,
                headers=headers,
                timeout=self.config.timeout_seconds,
                follow_redirects=False,
                trust_env=False,
                transport=self.transport,
            ) as client:
                with client.stream("POST" if is_create else "GET", endpoint, **kwargs) as response:
                    if response.status_code in {401, 403, 404}:
                        raise DomainError("github_access_denied", 403)
                    if response.status_code == 429:
                        raise DomainError("github_rate_limited", 429)
                    if response.status_code != (201 if is_create else 200):
                        raise DomainError("github_request_failed", 502)
                    raw = bytearray()
                    for chunk in response.iter_bytes():
                        if time.monotonic() > deadline:
                            raise DomainError("github_timeout", 504)
                        raw.extend(chunk)
                        if len(raw) > 262144:
                            raise DomainError("github_result_too_large", 502)
                    import json

                    payload = json.loads(raw)
        except httpx2.TimeoutException:
            raise DomainError("github_timeout", 504) from None
        except (httpx2.HTTPError, ValueError, TypeError):
            raise DomainError("github_response_invalid", 502) from None
        if is_create:
            return {"issue": self._issue(payload, repository)}
        if not isinstance(payload, list):
            raise DomainError("github_response_invalid", 502)
        return {
            "issues": [
                self._issue(item, repository)
                for item in payload
                if isinstance(item, dict) and "pull_request" not in item
            ][: parameters.get("limit", 10)]
        }

    def _issue(self, item, repository):
        if not isinstance(item, dict):
            raise DomainError("github_response_invalid", 502)
        number = item.get("number")
        url = item.get("html_url", "")
        if (
            type(number) is not int
            or number < 1
            or not isinstance(url, str)
            or urlsplit(url).scheme != "https"
            or urlsplit(url).netloc != "github.com"
            or urlsplit(url).query
            or urlsplit(url).fragment
            or urlsplit(url).path.lower() != f"/{repository.lower()}/issues/{number}"
        ):
            raise DomainError("github_response_invalid", 502)
        return {"number": number, "title": item.get("title"), "url": url, "state": item.get("state")}

    def receipt(self, result):
        issue = result.get("issue")
        return (
            {"provider": "github", "issue_number": issue["number"], "issue_url": issue["url"]}
            if issue
            else {"provider": "github", "issue_count": len(result["issues"])}
        )
