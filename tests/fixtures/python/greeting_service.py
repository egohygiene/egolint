"""Greeting formatting services."""

from __future__ import annotations


def format_greeting(name: str) -> str:
    """Format a greeting for a supplied name."""
    normalized_name = name.strip()

    if not normalized_name:
        return "Hello."

    return f"Hello, {normalized_name}."
