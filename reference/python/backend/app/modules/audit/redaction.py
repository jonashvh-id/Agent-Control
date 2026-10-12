"""Adapted ZNode safe summaries; values and arbitrary parameter names are never stored."""

from typing import Any


def summarize_input(value: dict[str, Any]) -> dict[str, Any]:
    # Even a parameter name can contain a pasted secret. Record only shape and types.
    return {
        "parameter_count": len(value),
        "value_types": sorted(type(v).__name__ for v in value.values()),
        "values_recorded": False,
    }
