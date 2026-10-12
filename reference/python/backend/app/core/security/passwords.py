from __future__ import annotations

import base64
import hashlib
import hmac
import os

SCRYPT_N = 2**14
SCRYPT_R = 8
SCRYPT_P = 1
SALT_BYTES = 16
MIN_PASSWORD_LENGTH = 12


class PasswordError(ValueError):
    """Raised when a password does not meet the Auth V1 baseline."""


def validate_password(password: str) -> str:
    value = password or ""
    if len(value) < MIN_PASSWORD_LENGTH:
        raise PasswordError("Password must be at least 12 characters.")
    if len(value) > 256:
        raise PasswordError("Password must be 256 characters or fewer.")
    if any(character.isspace() for character in value):
        raise PasswordError("Password must not contain spaces or line breaks.")
    if (
        not any(character.islower() for character in value)
        or not any(character.isupper() for character in value)
        or not any(character.isdigit() for character in value)
    ):
        raise PasswordError(
            "Password must include at least one uppercase letter, one lowercase letter, and one digit."
        )
    return value


def hash_password(password: str) -> str:
    normalized = validate_password(password)
    salt = os.urandom(SALT_BYTES)
    derived = hashlib.scrypt(
        normalized.encode("utf-8"),
        salt=salt,
        n=SCRYPT_N,
        r=SCRYPT_R,
        p=SCRYPT_P,
    )
    salt_b64 = base64.b64encode(salt).decode("ascii")
    derived_b64 = base64.b64encode(derived).decode("ascii")
    return f"scrypt${SCRYPT_N}${SCRYPT_R}${SCRYPT_P}${salt_b64}${derived_b64}"


def verify_password(password: str, stored_hash: str) -> bool:
    if not password or not stored_hash:
        return False

    try:
        algorithm, n_raw, r_raw, p_raw, salt_b64, digest_b64 = stored_hash.split("$", 5)
    except ValueError:
        return False

    if algorithm != "scrypt":
        return False

    try:
        salt = base64.b64decode(salt_b64.encode("ascii"))
        expected = base64.b64decode(digest_b64.encode("ascii"))
        derived = hashlib.scrypt(
            password.encode("utf-8"),
            salt=salt,
            n=int(n_raw),
            r=int(r_raw),
            p=int(p_raw),
        )
    except (ValueError, TypeError):
        return False

    return hmac.compare_digest(derived, expected)
