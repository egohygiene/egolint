"""Tests for greeting formatting services."""

from greeting_service import format_greeting


def test_formats_named_greeting() -> None:
    """Format a greeting containing a supplied name."""
    assert format_greeting("Ego Hygiene") == "Hello, Ego Hygiene."


def test_formats_empty_greeting() -> None:
    """Format a generic greeting when no name is provided."""
    assert format_greeting("   ") == "Hello."
