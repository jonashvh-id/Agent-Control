from backend.app.modules.connectors.models import AgentToolDefinition

ISSUE = {
    "type": "object",
    "properties": {
        "number": {"type": "integer", "minimum": 1},
        "title": {"type": "string", "maxLength": 1024},
        "url": {"type": "string", "maxLength": 2048},
        "state": {"type": "string", "enum": ["open", "closed"]},
    },
    "required": ["number", "title", "url", "state"],
    "additionalProperties": False,
}


def definitions(config):
    repository = {"type": "string", "enum": list(config.repositories)}
    listing = {
        "type": "object",
        "properties": {
            "repository": repository,
            "state": {"type": "string", "enum": ["open", "closed", "all"]},
            "limit": {"type": "integer", "minimum": 1, "maximum": 20},
            "page": {"type": "integer", "minimum": 1, "maximum": 100},
        },
        "required": ["repository"],
        "additionalProperties": False,
    }
    creation = {
        "type": "object",
        "properties": {
            "repository": repository,
            "title": {"type": "string", "minLength": 1, "maxLength": 256, "pattern": r"\S"},
            "body": {"type": "string", "maxLength": 12000},
        },
        "required": ["repository", "title", "body"],
        "additionalProperties": False,
    }
    yield AgentToolDefinition(
        "issues.list",
        "Read issues in explicitly configured GitHub repositories.",
        listing,
        {
            "type": "object",
            "properties": {"issues": {"type": "array", "items": ISSUE, "maxItems": 20}},
            "required": ["issues"],
            "additionalProperties": False,
        },
        ("issues:read",),
        "low",
        False,
    )
    yield AgentToolDefinition(
        "issues.create",
        "Create one GitHub issue after exact human approval.",
        creation,
        {
            "type": "object",
            "properties": {"issue": ISSUE},
            "required": ["issue"],
            "additionalProperties": False,
        },
        ("issues:write",),
        "medium",
        True,
    )
