"""Consumer fixture with one intentional Ruff finding."""

import json  # noqa: F401 -- ignored only by the repository's own lint pass


def greeting(name: str) -> str:
    """Return a friendly greeting."""
    return f"Hello, {name}!"
