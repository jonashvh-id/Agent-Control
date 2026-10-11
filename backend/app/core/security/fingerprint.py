"""ZNode canonical JSON request hashing, tightened to strict JSON and keyed digests."""

import hashlib
import hmac
import json


def canonical_json(value: object) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False, ensure_ascii=False)


def fingerprint(value: object, secret: str) -> str:
    # Keying avoids offline dictionary attacks against low-entropy sensitive parameters.
    return hmac.new(secret.encode(), canonical_json(value).encode(), hashlib.sha256).hexdigest()
