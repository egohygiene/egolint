#!/usr/bin/env bash

# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

set -o errexit
set -o nounset
set -o pipefail

readonly CONFIGURATION_PATH=".config/lint/latex/latexindent.yml"

# @description Print the command contract for the latexindent runner.
print_usage() {
  printf '%s\n' \
    "Usage: latexindent.sh <check|format> [file]" \
    "" \
    "Modes:" \
    "  check   Verify every first-party LaTeX file." \
    "  format  Format one explicitly supplied LaTeX file."
}

# @description Return whether a path belongs to generated, vendored, or fixture data.
# @arg $1 string Repository-relative path.
is_excluded_path() {
  case "$1" in
  ./.git/* | ./.cache/* | ./.reports/* | */node_modules/* | */vendor/* | */.venv/* | */venv/* | */dist/* | */build/* | */coverage/* | */target/* | */tests/fixtures/*)
    return 0
    ;;
  *)
    return 1
    ;;
  esac
}

# @description Render one file through the canonical latexindent configuration.
# @arg $1 string Input file.
# @arg $2 string Output file.
render_file() {
  local input_file="$1"
  local output_file="$2"

  latexindent \
    --local="${CONFIGURATION_PATH}" \
    "${input_file}" \
    >"${output_file}"

  if test ! -s "${output_file}"; then
    printf 'latexindent produced empty output: %s\n' "${input_file}" >&2
    return 1
  fi
}

# @description Check every applicable first-party LaTeX file.
check_files() {
  local discovery_file
  local status=0
  local latex_file
  local temporary_file

  discovery_file="$(mktemp "${TMPDIR:-/tmp}/latexindent-files.XXXXXX")"
  find . -type f \
    \( -name "*.tex" -o -name "*.sty" -o -name "*.cls" -o -name "*.bib" \) \
    -print0 \
    >"${discovery_file}"

  while IFS= read -r -d '' latex_file; do
    # This predicate intentionally participates in control flow.
    # shellcheck disable=SC2310
    if is_excluded_path "${latex_file}"; then
      continue
    fi

    temporary_file="$(mktemp "${TMPDIR:-/tmp}/latexindent-check.XXXXXX")"
    # A render failure is accumulated so every applicable file is checked.
    # shellcheck disable=SC2310
    if ! render_file "${latex_file}" "${temporary_file}"; then
      status=1
    elif ! cmp --silent "${latex_file}" "${temporary_file}"; then
      printf 'Formatting required: %s\n' "${latex_file}"
      status=1
    fi
    rm --force "${temporary_file}"
  done <"${discovery_file}"

  rm --force "${discovery_file}"

  return "${status}"
}

# @description Format one explicitly supplied file through a temporary output.
# @arg $1 string Input file.
format_file() {
  local input_file="$1"
  local temporary_file

  if test ! -f "${input_file}"; then
    printf 'The requested LaTeX file does not exist: %s\n' "${input_file}" >&2
    return 2
  fi

  temporary_file="$(mktemp "${TMPDIR:-/tmp}/latexindent-format.XXXXXX")"
  # Failure is handled here so the temporary file can be removed first.
  # shellcheck disable=SC2310
  if ! render_file "${input_file}" "${temporary_file}"; then
    rm --force "${temporary_file}"
    return 1
  fi

  if cmp --silent "${input_file}" "${temporary_file}"; then
    rm --force "${temporary_file}"
    printf 'Already formatted: %s\n' "${input_file}"
    return 0
  fi

  cp "${temporary_file}" "${input_file}"
  rm --force "${temporary_file}"
  printf 'Formatted: %s\n' "${input_file}"
}

# @description Dispatch the latexindent runner.
# @arg $1 string Operation: check or format.
# @arg $2 string Optional file for format mode.
main() {
  if ! command -v latexindent >/dev/null 2>&1; then
    printf '%s\n' "latexindent 4.0 is required on PATH." >&2
    return 3
  fi

  case "${1:-}" in
  check)
    check_files
    ;;
  format)
    if test -z "${2:-}"; then
      print_usage >&2
      return 2
    fi
    format_file "$2"
    ;;
  *)
    print_usage >&2
    return 2
    ;;
  esac
}

main "$@"
