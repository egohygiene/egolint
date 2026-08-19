"""Safe DevSkim integration fixture."""

from hashlib import sha256


def hash_identifier(identifier: str) -> str:
    """Return a deterministic SHA-256 digest for a non-secret identifier."""
    return sha256(identifier.encode("utf-8")).hexdigest()
