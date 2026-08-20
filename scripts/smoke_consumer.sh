#!/usr/bin/env bash

# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

set -o errexit
set -o nounset
set -o pipefail

if [[ $# -ne 1 ]]; then
  printf "Usage: %s IMAGE\n" "$(basename "$0")" >&2
  exit 2
fi

IMAGE="$1"
REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
TEMPORARY_ROOT="$(mktemp -d)"
TARGET_DIRECTORY="${TEMPORARY_ROOT}/target"
readonly IMAGE REPOSITORY_ROOT TEMPORARY_ROOT TARGET_DIRECTORY

cleanup() {
  rm -rf -- "${TEMPORARY_ROOT}"
}
trap cleanup EXIT

cargo build --locked --release \
  --manifest-path "${REPOSITORY_ROOT}/Cargo.toml" \
  --target-dir "${TARGET_DIRECTORY}" \
  --bin "egolint"

run_fixture() {
  local name="$1"
  local expected_status="$2"
  local workspace="${TEMPORARY_ROOT}/${name}"
  local status=0

  cp -R "${REPOSITORY_ROOT}/tests/fixtures/consumers/${name}" "${workspace}"
  git -C "${workspace}" init --quiet
  git -C "${workspace}" -c "user.name=Egolint Smoke" \
    -c "user.email=egolint@invalid.example" \
    add --all --
  git -C "${workspace}" -c "user.name=Egolint Smoke" \
    -c "user.email=egolint@invalid.example" \
    commit --quiet --no-gpg-sign --message "Consumer fixture"
  set +o errexit
  "${TARGET_DIRECTORY}/release/egolint" \
    --workspace "${workspace}" \
    lint \
    --profile "fast" \
    --runtime "docker" \
    --image "${IMAGE}" \
    --pull-policy "never" \
    --network "none" \
    --megalinter-config ".mega-linter.yml"
  status=$?
  set -o errexit

  [[ ${status} -eq ${expected_status} ]] || {
    printf "fixture %s returned %s; expected %s\n" "${name}" "${status}" "${expected_status}" >&2
    return 1
  }
  [[ -s "${workspace}/.reports/egolint/run.json" ]] || {
    printf "fixture %s did not produce run.json\n" "${name}" >&2
    return 1
  }
  [[ -s "${workspace}/.reports/egolint/egolint.sarif" ]] || {
    printf "fixture %s did not produce egolint.sarif\n" "${name}" >&2
    return 1
  }
}

run_fixture "clean" 0
run_fixture "findings" 1
