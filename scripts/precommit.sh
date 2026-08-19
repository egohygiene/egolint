#!/usr/bin/env bash

# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

set -o errexit
set -o nounset
set -o pipefail

readonly PRE_COMMIT_VERSION="4.6.0"

# @description Print the command contract for this pre-commit runner.
print_usage() {
  printf '%s\n' \
    "Usage: precommit.sh <staged|all> [hook-id]" \
    "" \
    "Modes:" \
    "  staged  Run only the automatic structural/security profile." \
    "  all     Run the complete manual profile against all tracked files."
}

# @description Execute pre-commit through an installed command, Python module, or uvx.
# @arg $@ string Arguments forwarded to the pre-commit CLI.
run_pre_commit() {
  if command -v pre-commit >/dev/null 2>&1; then
    pre-commit "$@"
    return
  fi

  if python3 -c "import pre_commit" >/dev/null 2>&1; then
    python3 -m pre_commit "$@"
    return
  fi

  if command -v uvx >/dev/null 2>&1; then
    uvx --from "pre-commit==${PRE_COMMIT_VERSION}" pre-commit "$@"
    return
  fi

  printf '%s\n' \
    "pre-commit ${PRE_COMMIT_VERSION} is required." \
    "Install the Egolint Python development dependencies or install uv." >&2
  return 3
}

# @description Build the stage-specific pre-commit invocation.
# @arg $1 string Execution mode: staged or all.
# @arg $2 string Optional hook identifier.
main() {
  local mode="${1:-}"
  local hook_identifier="${2:-}"
  local repository_root
  repository_root="$(git rev-parse --show-toplevel)"
  cd "${repository_root}"

  local -a arguments=(
    run
  )

  if [[ -n ${hook_identifier} ]]; then
    arguments+=("${hook_identifier}")
  fi

  arguments+=(
    --config
    .pre-commit-config.yaml
  )

  case "${mode}" in
  staged)
    arguments+=(--hook-stage pre-commit)
    ;;
  all)
    arguments+=(--all-files --hook-stage manual)
    ;;
  *)
    print_usage >&2
    return 2
    ;;
  esac

  run_pre_commit "${arguments[@]}"
}

main "$@"
