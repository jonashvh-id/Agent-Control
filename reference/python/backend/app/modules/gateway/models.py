from dataclasses import dataclass
from typing import Any

from pydantic import BaseModel, ConfigDict, Field, field_validator


class ExecutionProposal(BaseModel):
    model_config = ConfigDict(extra="forbid", strict=True)
    connector: str = Field(pattern=r"^[a-z][a-z0-9_-]{0,63}$")
    tool: str = Field(pattern=r"^[a-z][a-z0-9_.:-]{0,127}$")
    parameters: dict[str, Any] = Field(default_factory=dict)
    idempotency_key: str = Field(min_length=8, max_length=128, pattern=r"^[A-Za-z0-9_-]+$")
    approval_id: str | None = Field(default=None, pattern=r"^[a-f0-9]{32}$")

    @field_validator("parameters")
    @classmethod
    def json_only(cls, value):
        from backend.app.core.security.fingerprint import canonical_json

        stack = [(value, 0)]
        count = 0
        while stack:
            item, depth = stack.pop()
            count += 1
            if depth > 32 or count > 1000:
                raise ValueError("Parameters exceed the foundation complexity limit")
            if isinstance(item, dict):
                stack.extend((child, depth + 1) for child in item.values())
            elif isinstance(item, list):
                stack.extend((child, depth + 1) for child in item)
        try:
            serialized = canonical_json(value)
        except (TypeError, ValueError, RecursionError) as exc:
            raise ValueError("Parameters must be finite JSON") from exc
        if len(serialized.encode()) > 16384:
            raise ValueError("Parameters exceed the foundation size limit")
        return value

    def binding(self):
        return {
            "connector": self.connector,
            "tool": self.tool,
            "parameters": self.parameters,
            "idempotency_key": self.idempotency_key,
        }


@dataclass(frozen=True)
class GatewayOutcome:
    status_code: int
    body: dict
