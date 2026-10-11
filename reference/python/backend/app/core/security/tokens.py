"""Bearer generation and HMAC hashing extracted from ZNode; independent key and namespace."""

import hashlib
import hmac
import secrets

AGENT_TOKEN_PREFIX = "ac_agent"


def generate_token() -> str:
    return f"{AGENT_TOKEN_PREFIX}_{secrets.token_urlsafe(32)}"


def hash_token(token: str, secret_key: str) -> str:
    return hmac.new(secret_key.encode("utf-8"), token.strip().encode("utf-8"), hashlib.sha256).hexdigest()
