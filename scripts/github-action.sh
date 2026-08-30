#!/usr/bin/env bash

# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

set -o errexit
set -o nounset
set -o pipefail

readonly REPORT_ROOT=".reports/egolint"
MATCHER_INSTALLED="false"
COMMANDS_STOPPED="false"
COMMAND_TOKEN=""

fail() {
  printf "egolint-action: %s\n" "$1" >&2
  exit 2
}

remove_matcher() {
  if [[ ${MATCHER_INSTALLED} == "true" ]]; then
    printf "::remove-matcher owner=egolint::\n"
  fi
}

stop_workflow_commands() {
  IFS= read -r COMMAND_TOKEN <"/proc/sys/kernel/random/uuid" ||
    fail "could not allocate a workflow-command guard"
  [[ ${COMMAND_TOKEN} =~ ^[0-9a-f-]{36}$ ]] ||
    fail "workflow-command guard was malformed"
  printf "::stop-commands::%s\n" "${COMMAND_TOKEN}"
  COMMANDS_STOPPED="true"
}

resume_workflow_commands() {
  if [[ ${COMMANDS_STOPPED} == "true" ]]; then
    printf "::%s::\n" "${COMMAND_TOKEN}"
    COMMANDS_STOPPED="false"
  fi
}

cleanup() {
  resume_workflow_commands
  remove_matcher
}

require_choice() {
  local name="$1"
  local value="$2"
  shift 2
  local allowed=""
  for allowed in "$@"; do
    [[ ${value} == "${allowed}" ]] && return 0
  done
  fail "invalid ${name}: ${value}"
}

require_boolean() {
  require_choice "$1" "$2" "true" "false"
}

reject_controls() {
  local name="$1"
  local value="$2"
  if [[ ${value} =~ [[:cntrl:]] ]]; then
    fail "${name} may not contain control characters"
  fi
}

workspace_input="${INPUT_WORKSPACE:-.}"
profile="${INPUT_PROFILE:-fast}"
image="${INPUT_IMAGE:-}"
allow_unpinned="${INPUT_ALLOW_UNPINNED_IMAGE:-false}"
pull_policy="${INPUT_PULL_POLICY:-always}"
network="${INPUT_NETWORK:-none}"
megalinter_config="${INPUT_MEGALINTER_CONFIG:-}"
repository_contract="${INPUT_REPOSITORY_CONTRACT:-}"
repository_intelligence="${INPUT_REPOSITORY_INTELLIGENCE:-}"
repository_presentation="${INPUT_REPOSITORY_PRESENTATION:-}"
represented_commit="${INPUT_REPRESENTED_COMMIT:-}"
suppression="${INPUT_SUPPRESSION:-}"
evaluation_date="${INPUT_EVALUATION_DATE:-}"
changed_only="${INPUT_CHANGED_ONLY:-true}"

[[ -n ${GITHUB_WORKSPACE:-} ]] || fail "GITHUB_WORKSPACE is required"
[[ -n ${GITHUB_ACTION_PATH:-} ]] || fail "GITHUB_ACTION_PATH is required"
[[ -n ${RUNNER_TEMP:-} ]] || fail "RUNNER_TEMP is required"
[[ -n ${GITHUB_OUTPUT:-} ]] || fail "GITHUB_OUTPUT is required"
[[ -n ${HOME:-} ]] || fail "HOME is required"
[[ ${RUNNER_OS:-} == "Linux" ]] || fail "this composite action supports Linux runners only"
reject_controls "GITHUB_WORKSPACE" "${GITHUB_WORKSPACE}"
reject_controls "GITHUB_ACTION_PATH" "${GITHUB_ACTION_PATH}"
reject_controls "workspace" "${workspace_input}"
reject_controls "megalinter-config" "${megalinter_config}"
reject_controls "repository-contract" "${repository_contract}"
reject_controls "repository-intelligence" "${repository_intelligence}"
reject_controls "repository-presentation" "${repository_presentation}"
reject_controls "represented-commit" "${represented_commit}"
reject_controls "suppression" "${suppression}"
reject_controls "evaluation-date" "${evaluation_date}"
[[ -n ${image} ]] || fail "input 'image' is required"
[[ ${profile} =~ ^[a-z0-9][a-z0-9-]*$ ]] || fail "profile contains unsupported characters"
require_boolean "allow-unpinned-image" "${allow_unpinned}"
require_boolean "changed-only" "${changed_only}"
require_choice "pull-policy" "${pull_policy}" "always" "missing" "never"
require_choice "network" "${network}" "none" "bridge"

case "${workspace_input}" in
  "" | /* | ../* | */../* | */..) fail "workspace must be repository-relative and may not traverse upward" ;;
esac

workspace="$(realpath --canonicalize-existing "${GITHUB_WORKSPACE}/${workspace_input}")"
github_workspace="$(realpath --canonicalize-existing "${GITHUB_WORKSPACE}")"
case "${workspace}" in
  "${github_workspace}" | "${github_workspace}"/*) ;;
  *) fail "workspace resolves outside GITHUB_WORKSPACE" ;;
esac
[[ -d ${workspace} ]] || fail "workspace is not a directory"

for policy_path in "${megalinter_config}" "${repository_contract}" "${repository_intelligence}" "${repository_presentation}" "${suppression}"; do
  if [[ -n ${policy_path} ]]; then
    case "${policy_path}" in
      /* | ../* | */../* | */..) fail "policy inputs must be workspace-relative and may not traverse upward" ;;
    esac
  fi
done
if [[ -n ${suppression} || -n ${evaluation_date} ]]; then
  [[ -n ${suppression} && -n ${evaluation_date} ]] ||
    fail "suppression and evaluation-date must be supplied together"
  [[ ${evaluation_date} =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] ||
    fail "evaluation-date must use YYYY-MM-DD"
fi
if [[ -n ${repository_intelligence} || -n ${repository_presentation} ]]; then
  represented_commit="${represented_commit:-${GITHUB_SHA:-}}"
  [[ -n ${represented_commit} ]] ||
    fail "represented-commit or GITHUB_SHA is required with semantic repository validation"
  if [[ ${represented_commit} != "unknown" && ${represented_commit} != "not-applicable" ]]; then
    [[ ${represented_commit} =~ ^[0-9a-f]{40}$ ]] ||
      fail "represented-commit must be a full lowercase Git SHA, unknown, or not-applicable"
  fi
elif [[ -n ${represented_commit} ]]; then
  fail "represented-commit requires repository-intelligence or repository-presentation"
fi

if [[ ${allow_unpinned} != "true" && ! ${image} =~ ^[A-Za-z0-9][A-Za-z0-9._/:@+-]*@sha256:[0-9a-f]{64}$ ]]; then
  fail "image must be pinned by sha256 digest"
fi
if [[ ${allow_unpinned} == "true" ]]; then
  if [[ ! ${image} =~ ^[A-Za-z0-9][A-Za-z0-9._-]*(:[A-Za-z0-9_][A-Za-z0-9_.-]*)?$ ]]; then
    fail "an unpinned smoke-test image must be a local name without a registry or digest"
  fi
  [[ ${pull_policy} == "never" ]] ||
    fail "an unpinned smoke-test image requires pull-policy 'never'"
fi

relative_workspace="${workspace#"${github_workspace}"/}"
[[ ${workspace} == "${github_workspace}" ]] && relative_workspace="."
reject_controls "resolved workspace" "${relative_workspace}"
report_prefix="${relative_workspace%/}/${REPORT_ROOT}"
[[ ${relative_workspace} == "." ]] && report_prefix="${REPORT_ROOT}"

{
  printf "run-report=%s/run.json\n" "${report_prefix}"
  printf "sarif-report=%s/egolint.sarif\n" "${report_prefix}"
  printf "repository-intelligence-report=%s/repository-intelligence.json\n" "${report_prefix}"
  printf "repository-presentation-report=%s/repository-presentation.json\n" "${report_prefix}"
  printf "debt-json=%s/debt.json\n" "${report_prefix}"
  printf "debt-markdown=%s/debt.md\n" "${report_prefix}"
  printf "raw-megalinter-json=%s/mega-linter-report.json\n" "${report_prefix}"
  printf "raw-megalinter-sarif=%s/mega-linter-report.sarif\n" "${report_prefix}"
} >>"${GITHUB_OUTPUT}"

target_directory="${RUNNER_TEMP}/egolint-action-target"
printf "::add-matcher::%s\n" \
  "${GITHUB_ACTION_PATH}/integrations/github/egolint-problem-matcher.json"
MATCHER_INSTALLED="true"
trap cleanup EXIT
cargo_environment=(
  "env" "-i"
  "CI=true"
  "HOME=${HOME}"
  "PATH=${PATH}"
  "CARGO_HOME=${RUNNER_TEMP}/egolint-action-cargo-home"
)
[[ -z ${RUSTUP_HOME:-} ]] || cargo_environment+=("RUSTUP_HOME=${RUSTUP_HOME}")
stop_workflow_commands
(
  cd "${GITHUB_ACTION_PATH}"
  "${cargo_environment[@]}" cargo build --locked --release \
    --manifest-path "${GITHUB_ACTION_PATH}/Cargo.toml" \
    --target-dir "${target_directory}" \
    --bin "egolint"
)
resume_workflow_commands

command=(
  "${target_directory}/release/egolint"
  "--workspace" "${workspace}"
  "lint"
  "--profile" "${profile}"
  "--runtime" "docker"
  "--image" "${image}"
  "--pull-policy" "${pull_policy}"
  "--network" "${network}"
)
[[ ${changed_only} == "true" ]] && command+=("--changed-only")
[[ -z ${megalinter_config} ]] || command+=("--megalinter-config" "${megalinter_config}")
[[ -z ${repository_contract} ]] || command+=("--repository-contract" "${repository_contract}")
if [[ -n ${repository_intelligence} ]]; then
  command+=("--repository-intelligence" "${repository_intelligence}")
fi
if [[ -n ${repository_presentation} ]]; then
  command+=("--repository-presentation" "${repository_presentation}")
fi
[[ -z ${repository_intelligence} && -z ${repository_presentation} ]] ||
  command+=("--represented-commit" "${represented_commit}")
if [[ -n ${suppression} ]]; then
  command+=("--suppression" "${suppression}" "--evaluation-date" "${evaluation_date}")
fi

runtime_environment=("env" "-i" "CI=true" "HOME=${HOME}" "PATH=${PATH}")
for variable in DOCKER_API_VERSION DOCKER_CERT_PATH DOCKER_CONFIG DOCKER_CONTEXT DOCKER_HOST DOCKER_TLS_VERIFY; do
  [[ -z ${!variable:-} ]] || runtime_environment+=("${variable}=${!variable}")
done

set +o errexit
"${runtime_environment[@]}" "${command[@]}"
status=$?
set -o errexit
printf "exit-code=%s\n" "${status}" >>"${GITHUB_OUTPUT}"
exit "${status}"
