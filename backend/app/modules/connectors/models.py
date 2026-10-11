from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Literal

AgentToolRiskLevel = Literal["low", "medium", "high", "critical"]


@dataclass(frozen=True)
class AgentToolDefinition:
    name: str
    description: str
    input_schema: dict[str, Any]
    output_schema: dict[str, Any]
    required_scopes: tuple[str, ...]
    risk_level: AgentToolRiskLevel
    requires_confirmation: bool

    def to_dict(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "description": self.description,
            "input_schema": self.input_schema,
            "output_schema": self.output_schema,
            "required_scopes": list(self.required_scopes),
            "risk_level": self.risk_level,
            "requires_confirmation": self.requires_confirmation,
        }
