#!/usr/bin/env sh

# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

set -eu

# @description Run the repository-pinned pnpm through Corepack when available.
# @arg $@ string Arguments forwarded to pnpm.
if command -v corepack >/dev/null 2>&1; then
  exec corepack pnpm "$@"
fi

if command -v pnpm >/dev/null 2>&1; then
  exec pnpm "$@"
fi

printf '%s\n' "Corepack or pnpm is required to run Egolint's Node toolchain." >&2
exit 127
