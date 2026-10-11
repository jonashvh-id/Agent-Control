"""Map governed tools to MCP without duplicating policy or execution logic."""

from typing import Any

from pydantic import BaseModel, ConfigDict

from backend.app.modules.gateway.models import ExecutionProposal
from mcp import types


def mcp_tool(tool):
    return types.Tool(
        name=tool["connector"] + "." + tool["name"],
        description=tool["description"],
        inputSchema={
            "type": "object",
            "properties": {
                "parameters": tool["input_schema"],
                "idempotency_key": {
                    "type": "string",
                    "minLength": 8,
                    "maxLength": 128,
                    "pattern": r"^[A-Za-z0-9_-]+$",
                },
                "approval_id": {"type": "string", "pattern": r"^[a-f0-9]{32}$"},
            },
            "required": ["parameters", "idempotency_key"],
            "additionalProperties": False,
        },
        annotations=types.ToolAnnotations(
            readOnlyHint=all(s.endswith(":read") for s in tool["required_scopes"]), openWorldHint=True
        ),
    )


class MCPArguments(BaseModel):
    model_config = ConfigDict(extra="forbid", strict=True)
    parameters: dict[str, Any]
    idempotency_key: str
    approval_id: str | None = None


def proposal_for(name, arguments):
    connector, separator, tool = name.partition(".")
    if not separator:
        connector, tool = "unknown", "unknown"
    return ExecutionProposal.model_validate(
        {"connector": connector, "tool": tool, **MCPArguments.model_validate(arguments or {}).model_dump()}
    )
