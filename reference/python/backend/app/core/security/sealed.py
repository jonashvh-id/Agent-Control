"""Separate-purpose encryption for authorized approval previews; never audit payloads."""

import base64
import hashlib
import json

from cryptography.fernet import Fernet, InvalidToken

from backend.app.core.errors.domain import DomainError


class PreviewCipher:
    def __init__(self, secret):
        material = hashlib.sha256(("approval-preview-v1:" + secret).encode()).digest()
        self.cipher = Fernet(base64.urlsafe_b64encode(material))

    def seal(self, value):
        return self.cipher.encrypt(json.dumps(value, ensure_ascii=False, allow_nan=False).encode()).decode()

    def open(self, value):
        if not value:
            return None
        try:
            return json.loads(self.cipher.decrypt(value.encode()))
        except (InvalidToken, ValueError):
            raise DomainError("approval_preview_unavailable", 409) from None
