from dataclasses import dataclass
from typing import Protocol

from jsonschema import Draft202012Validator

from .models import AgentToolDefinition


def validate_local_schema(schema):
    Draft202012Validator.check_schema(schema)
    stack = [schema]
    while stack:
        item = stack.pop()
        if isinstance(item, dict):
            for key in ("$ref", "$dynamicRef"):
                if key in item and not str(item[key]).startswith("#"):
                    raise ValueError("Remote schema references are forbidden")
            stack.extend(item.values())
        elif isinstance(item, list):
            stack.extend(item)


class ConnectorAdapter(Protocol):
    """Trusted, explicitly registered code only. No credentials in proposals.

    Implementations must bound I/O, validate destinations, disable blind retries and
    treat returned content as untrusted. Cancellation is not implemented in v0.1.
    """

    def execute(self, tool: str, parameters: dict, *, idempotency_key: str) -> dict: ...


@dataclass(frozen=True)
class RegisteredTool:
    connector: str
    definition: AgentToolDefinition
    adapter: ConnectorAdapter


class ConnectorRegistry:
    def __init__(self):
        self._tools = {}

    def register(self, connector, definition, adapter):
        key = (connector, definition.name)
        if key in self._tools:
            raise ValueError("Duplicate tool registration")
        validate_local_schema(definition.input_schema)
        validate_local_schema(definition.output_schema)
        self._tools[key] = RegisteredTool(connector, definition, adapter)

    def get(self, connector, tool):
        return self._tools.get((connector, tool))

    def all(self):
        return list(self._tools.values())
