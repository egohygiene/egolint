# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

"""Positive Vulture fixture."""


def greeting(name: str) -> str:
    """Return a greeting."""

    return f"Hello, {name}."


MESSAGE = greeting("developer")
