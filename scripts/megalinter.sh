#!/usr/bin/env bash

# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

# megalinter
#
# A portable local wrapper around the MegaLinter container image. It is
# intentionally suitable for direct use, Taskfile tasks, and CI jobs.
#
# MegaLinter documentation: https://megalinter.io/latest/

if [[ ${BASH_SOURCE[0]} == "${0}" ]]; then
  set -o errexit
  set -o nounset
  set -o pipefail
fi

SCRIPT_NAME="$(basename "${BASH_SOURCE[0]}")"
readonly SCRIPT_NAME
readonly CONTAINER_WORKSPACE="/tmp/lint"
readonly DEFAULT_REGISTRY="ghcr.io/oxsecurity"
readonly DEFAULT_VERSION="v10.0.0"
readonly DEFAULT_REPORT_DIRECTORY=".reports/egolint"

readonly EXIT_SUCCESS=0
readonly EXIT_USAGE=2
readonly EXIT_DEPENDENCY=3
readonly EXIT_RUNTIME=4

RUNTIME="auto"
WORKSPACE=""
CONFIG_FILE="${MEGALINTER_CONFIG:-}"
FLAVOR="all"
MEGALINTER_VERSION="${MEGALINTER_VERSION:-${DEFAULT_VERSION}}"
IMAGE="${MEGALINTER_IMAGE:-}"
PULL_POLICY="missing"
TTY_MODE="auto"
CHANGED_ONLY="false"
FIX_VALUE=""
ENABLE_DESCRIPTORS=""
ENABLE_LINTERS=""
DISABLE_DESCRIPTORS=""
DISABLE_LINTERS=""
REPORT_DIRECTORY="${MEGALINTER_REPORT_DIRECTORY:-${DEFAULT_REPORT_DIRECTORY}}"
ENV_FILE=""
DOCKER_SOCKET="false"
USER_MODE="default"
PLATFORM=""
DEBUG_MODE="false"
QUIET_MODE="false"
DRY_RUN="false"
DOCTOR_MODE="false"

EXTRA_ENVS=()
EXTRA_VOLUMES=()
RUNTIME_ARGS=()
RUN_COMMAND=()

# @description Write an informational wrapper message unless quiet mode is active.
# @arg $@ string Message fragments.
log_info() {
  if [[ ${QUIET_MODE} != "true" ]]; then
    printf "[%s] %s\n" "${SCRIPT_NAME}" "$*"
  fi
}

# @description Write a diagnostic message when debug mode is active.
# @arg $@ string Message fragments.
log_debug() {
  if [[ ${DEBUG_MODE} == "true" ]]; then
    printf "[%s][debug] %s\n" "${SCRIPT_NAME}" "$*" >&2
  fi
}

# @description Write a warning message to standard error.
# @arg $@ string Message fragments.
log_warn() {
  printf "[%s][warning] %s\n" "${SCRIPT_NAME}" "$*" >&2
}

# @description Write an error message to standard error.
# @arg $@ string Message fragments.
log_error() {
  printf "[%s][error] %s\n" "${SCRIPT_NAME}" "$*" >&2
}

# @description Write a fatal error and exit with an explicit status.
# @arg $1 string Error message.
# @arg $2 integer Optional exit status; defaults to the usage status.
die() {
  local message="$1"
  local exit_code="${2:-${EXIT_USAGE}}"
  log_error "${message}"
  exit "${exit_code}"
}

# @description Print the wrapper command reference.
# @stdout Usage, option, environment, and example documentation.
show_help() {
  cat <<'EOF'
Usage:
  megalinter [options] [-- runtime-argument ...]

Run MegaLinter against a repository using Docker or Podman. With no options,
the wrapper discovers the Git root, uses .mega-linter.yml when present, and
runs the complete codebase with MegaLinter v10.0.0.

Selection:
  --descriptors LIST          Enable descriptor keys, such as PYTHON,YAML.
  --linters LIST              Enable canonical linter keys, such as
                              PYTHON_RUFF,YAML_YAMLLINT.
  --disable-descriptors LIST  Disable descriptor keys.
  --disable-linters LIST      Disable canonical linter keys.
  --changed-only              Validate only new or edited files.
  --fix[=LIST]                Apply fixes for all linters or a comma-separated
                              list of canonical linter keys.

Repository and output:
  --workspace PATH            Repository to lint. Default: Git root or cwd.
  --config PATH               MegaLinter config inside the workspace.
  --report-directory PATH     Repository-relative report directory.
                              Default: .reports/egolint.
  --no-reports                Disable report generation.

Container image:
  --runtime auto|docker|podman  Container engine. Default: auto.
  --flavor NAME                MegaLinter flavor. Default: all.
  --version TAG                Image tag. Default: v10.0.0.
  --image IMAGE                Exact image reference; overrides flavor/version.
  --pull always|missing|never  Image pull policy. Default: missing.
  --platform PLATFORM          Optional container platform, such as linux/amd64.
  --tty auto|always|never      TTY allocation policy. Default: auto.
  --user default|host          Run with the host UID:GID on POSIX systems.

Advanced container options:
  --env NAME=VALUE             Set a MegaLinter environment variable. Repeatable.
  --env-file PATH              Pass a container environment file.
  --volume SPEC                Add a volume mount. Repeatable.
  --mount-docker-socket        Mount the engine socket read/write. This grants
                               the container control of the host engine.
  --runtime-arg ARG            Pass one argument to docker/podman run. Repeatable.
  --                           Pass all remaining arguments to docker/podman run.

Diagnostics:
  --doctor                     Validate configuration and runtime readiness.
  --dry-run                    Print a redacted command without running it.
  --debug                      Print diagnostic details and enable DEBUG logging.
  --quiet                      Suppress informational wrapper messages.
  --help                       Show this help.
  --wrapper-version            Show wrapper and default MegaLinter versions.

Environment defaults:
  MEGALINTER_IMAGE             Exact default image reference.
  MEGALINTER_VERSION           Default image tag.
  MEGALINTER_CONFIG            Default repository-relative configuration file.
  MEGALINTER_REPORT_DIRECTORY  Default repository-relative report directory.

Examples:
  megalinter
  megalinter --descriptors "BASH,YAML,MARKDOWN"
  megalinter --linters "PYTHON_RUFF,YAML_PRETTIER"
  megalinter --changed-only
  megalinter --fix
  megalinter --flavor python --version v10.0.0
  megalinter --env "LOG_LEVEL=DEBUG" --dry-run

Taskfile example:
  lint:
    cmds:
      - ./scripts/megalinter.sh

  lint:fix:
    cmds:
      - ./scripts/megalinter.sh --fix
EOF
}

# @description Print wrapper and default MegaLinter versions.
# @stdout Version information.
show_version() {
  printf "%s wrapper 1.0.0\n" "${SCRIPT_NAME}"
  printf "Default MegaLinter version: %s\n" "${DEFAULT_VERSION}"
}

# @description Require a non-option value for a command-line option.
# @arg $1 string Option name.
# @arg $2 string Candidate value.
require_option_value() {
  local option="$1"
  local value="${2:-}"
  if [[ -z ${value} || ${value} == --* ]]; then
    die "${option} requires a value."
  fi
}

# @description Validate a comma-separated MegaLinter key list.
# @arg $1 string Option name used in diagnostics.
# @arg $2 string Normalized key list.
validate_list() {
  local option="$1"
  local value="$2"
  if [[ ! ${value} =~ ^[A-Za-z0-9_,-]+$ ]]; then
    die "${option} accepts only comma-separated MegaLinter keys."
  fi
}

# @description Remove whitespace and uppercase a MegaLinter key list.
# @arg $1 string Raw key list.
# @stdout Normalized comma-separated key list.
normalize_list() {
  printf "%s" "$1" | tr -d '[:space:]' | tr '[:lower:]' '[:upper:]'
}

# @description Parse wrapper arguments into validated global option state.
# @arg $@ string Wrapper command-line arguments.
parse_arguments() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
    --descriptors)
      require_option_value "$1" "${2:-}"
      ENABLE_DESCRIPTORS="$(normalize_list "$2")"
      validate_list "$1" "${ENABLE_DESCRIPTORS}"
      shift 2
      ;;
    --linters)
      require_option_value "$1" "${2:-}"
      ENABLE_LINTERS="$(normalize_list "$2")"
      validate_list "$1" "${ENABLE_LINTERS}"
      shift 2
      ;;
    --disable-descriptors)
      require_option_value "$1" "${2:-}"
      DISABLE_DESCRIPTORS="$(normalize_list "$2")"
      validate_list "$1" "${DISABLE_DESCRIPTORS}"
      shift 2
      ;;
    --disable-linters)
      require_option_value "$1" "${2:-}"
      DISABLE_LINTERS="$(normalize_list "$2")"
      validate_list "$1" "${DISABLE_LINTERS}"
      shift 2
      ;;
    --changed-only)
      CHANGED_ONLY="true"
      shift
      ;;
    --fix)
      FIX_VALUE="all"
      shift
      ;;
    --fix=*)
      FIX_VALUE="$(normalize_list "${1#*=}")"
      [[ -n ${FIX_VALUE} ]] || die "--fix requires a non-empty value after '='."
      validate_list "--fix" "${FIX_VALUE}"
      shift
      ;;
    --workspace)
      require_option_value "$1" "${2:-}"
      WORKSPACE="$2"
      shift 2
      ;;
    --config)
      require_option_value "$1" "${2:-}"
      CONFIG_FILE="$2"
      shift 2
      ;;
    --report-directory)
      require_option_value "$1" "${2:-}"
      REPORT_DIRECTORY="$2"
      shift 2
      ;;
    --no-reports)
      REPORT_DIRECTORY="none"
      shift
      ;;
    --runtime)
      require_option_value "$1" "${2:-}"
      RUNTIME="$2"
      shift 2
      ;;
    --flavor)
      require_option_value "$1" "${2:-}"
      FLAVOR="$2"
      shift 2
      ;;
    --version)
      require_option_value "$1" "${2:-}"
      MEGALINTER_VERSION="$2"
      shift 2
      ;;
    --image)
      require_option_value "$1" "${2:-}"
      IMAGE="$2"
      shift 2
      ;;
    --pull)
      require_option_value "$1" "${2:-}"
      PULL_POLICY="$2"
      shift 2
      ;;
    --platform)
      require_option_value "$1" "${2:-}"
      PLATFORM="$2"
      shift 2
      ;;
    --tty)
      require_option_value "$1" "${2:-}"
      TTY_MODE="$2"
      shift 2
      ;;
    --user)
      require_option_value "$1" "${2:-}"
      USER_MODE="$2"
      shift 2
      ;;
    --env)
      require_option_value "$1" "${2:-}"
      [[ $2 == *=* && $2 != =* ]] || die "--env requires NAME=VALUE."
      EXTRA_ENVS+=("$2")
      shift 2
      ;;
    --env-file)
      require_option_value "$1" "${2:-}"
      ENV_FILE="$2"
      shift 2
      ;;
    --volume)
      require_option_value "$1" "${2:-}"
      EXTRA_VOLUMES+=("$2")
      shift 2
      ;;
    --mount-docker-socket)
      DOCKER_SOCKET="true"
      shift
      ;;
    --runtime-arg)
      [[ $# -ge 2 ]] || die "--runtime-arg requires a value."
      RUNTIME_ARGS+=("$2")
      shift 2
      ;;
    --doctor)
      DOCTOR_MODE="true"
      shift
      ;;
    --dry-run)
      DRY_RUN="true"
      shift
      ;;
    --debug)
      DEBUG_MODE="true"
      shift
      ;;
    --quiet)
      QUIET_MODE="true"
      shift
      ;;
    --help)
      show_help
      exit "${EXIT_SUCCESS}"
      ;;
    --wrapper-version)
      show_version
      exit "${EXIT_SUCCESS}"
      ;;
    --)
      shift
      while [[ $# -gt 0 ]]; do
        RUNTIME_ARGS+=("$1")
        shift
      done
      ;;
    *)
      die "Unknown option: $1. Run ${SCRIPT_NAME} --help for usage."
      ;;
    esac
  done
}

# @description Resolve an existing directory to a physical absolute path.
# @arg $1 string Candidate directory.
# @stdout Physical absolute path when the directory exists.
absolute_directory() {
  local path="$1"
  [[ -d ${path} ]] || return 1
  (cd "${path}" && pwd -P)
}

# @description Resolve and validate the repository workspace.
resolve_workspace() {
  local candidate="${WORKSPACE}"
  if [[ -z ${candidate} ]]; then
    candidate="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
  fi

  WORKSPACE="$(absolute_directory "${candidate}")" ||
    die "Workspace does not exist or is not a directory: ${candidate}"

  [[ -r ${WORKSPACE} ]] || die "Workspace is not readable: ${WORKSPACE}"
}

# @description Determine whether an absolute path is inside the workspace.
# @arg $1 string Absolute path.
path_is_within_workspace() {
  local path="$1"
  case "${path}" in
  "${WORKSPACE}" | "${WORKSPACE}"/*) return 0 ;;
  *) return 1 ;;
  esac
}

# @description Resolve an explicit or conventional MegaLinter configuration.
resolve_config() {
  local candidate=""

  if [[ -n ${CONFIG_FILE} ]]; then
    case "${CONFIG_FILE}" in
    /*) candidate="${CONFIG_FILE}" ;;
    *) candidate="${WORKSPACE}/${CONFIG_FILE}" ;;
    esac
  elif [[ -f "${WORKSPACE}/.mega-linter.yml" ]]; then
    candidate="${WORKSPACE}/.mega-linter.yml"
  elif [[ -f "${WORKSPACE}/.mega-linter.yaml" ]]; then
    candidate="${WORKSPACE}/.mega-linter.yaml"
  fi

  if [[ -z ${candidate} ]]; then
    CONFIG_FILE=""
    return 0
  fi

  [[ -f ${candidate} ]] || die "MegaLinter config does not exist: ${candidate}"
  candidate="$(cd "$(dirname "${candidate}")" && pwd -P)/$(basename "${candidate}")"
  path_is_within_workspace "${candidate}" ||
    die "MegaLinter config must be inside the workspace: ${candidate}"
  CONFIG_FILE="${candidate}"
}

# @description Validate and normalize the repository-relative report directory.
validate_report_directory() {
  [[ ${REPORT_DIRECTORY} == "none" ]] && return 0
  [[ -n ${REPORT_DIRECTORY} ]] || die "Report directory cannot be empty."
  case "${REPORT_DIRECTORY}" in
  /*) die "Report directory must be relative to the workspace." ;;
  */../* | ../* | */..) die "Report directory may not contain a '..' segment: ${REPORT_DIRECTORY}" ;;
  esac
  REPORT_DIRECTORY="${REPORT_DIRECTORY#./}"
}

# @description Validate enumerated options and option-dependent prerequisites.
validate_options() {
  case "${RUNTIME}" in auto | docker | podman) ;; *) die "Invalid runtime: ${RUNTIME}" ;; esac
  case "${PULL_POLICY}" in always | missing | never) ;; *) die "Invalid pull policy: ${PULL_POLICY}" ;; esac
  case "${TTY_MODE}" in auto | always | never) ;; *) die "Invalid TTY mode: ${TTY_MODE}" ;; esac
  case "${USER_MODE}" in default | host) ;; *) die "Invalid user mode: ${USER_MODE}" ;; esac

  [[ ${FLAVOR} =~ ^[a-z0-9][a-z0-9_-]*$ ]] || die "Invalid flavor: ${FLAVOR}"
  [[ -n ${MEGALINTER_VERSION} ]] || die "MegaLinter version cannot be empty."

  if [[ ${CHANGED_ONLY} == "true" && ! -d "${WORKSPACE}/.git" ]]; then
    git -C "${WORKSPACE}" rev-parse --git-dir >/dev/null 2>&1 ||
      die "--changed-only requires a Git worktree."
  fi

  if [[ -n ${ENV_FILE} ]]; then
    [[ -f ${ENV_FILE} ]] || die "Environment file does not exist: ${ENV_FILE}"
    ENV_FILE="$(cd "$(dirname "${ENV_FILE}")" && pwd -P)/$(basename "${ENV_FILE}")"
  fi
}

# @description Select Docker or Podman and confirm the executable is available.
select_runtime() {
  if [[ ${RUNTIME} == "auto" ]]; then
    if command -v docker >/dev/null 2>&1; then
      RUNTIME="docker"
    elif command -v podman >/dev/null 2>&1; then
      RUNTIME="podman"
    else
      die "Neither Docker nor Podman is installed." "${EXIT_DEPENDENCY}"
    fi
  elif ! command -v "${RUNTIME}" >/dev/null 2>&1; then
    die "Container runtime is not installed: ${RUNTIME}" "${EXIT_DEPENDENCY}"
  fi
}

# @description Resolve the MegaLinter image from flavor and version defaults.
resolve_image() {
  [[ -n ${IMAGE} ]] && return 0
  if [[ ${FLAVOR} == "all" ]]; then
    IMAGE="${DEFAULT_REGISTRY}/megalinter:${MEGALINTER_VERSION}"
  else
    IMAGE="${DEFAULT_REGISTRY}/megalinter-${FLAVOR}:${MEGALINTER_VERSION}"
  fi
}

# @description Check whether the selected container runtime service is ready.
runtime_ready() {
  "${RUNTIME}" info >/dev/null 2>&1
}

# @description Check whether the resolved MegaLinter image exists locally.
image_exists() {
  if [[ ${RUNTIME} == "docker" ]]; then
    docker image inspect "${IMAGE}" >/dev/null 2>&1
  else
    podman image exists "${IMAGE}" >/dev/null 2>&1
  fi
}

# @description Enforce the configured container-image pull policy.
prepare_image() {
  if [[ ${PULL_POLICY} == "always" ]]; then
    log_info "Pulling ${IMAGE}"
    "${RUNTIME}" pull "${IMAGE}"
  elif [[ ${PULL_POLICY} == "missing" ]] && ! image_exists; then
    log_info "Image is not present locally; pulling ${IMAGE}"
    "${RUNTIME}" pull "${IMAGE}"
  elif [[ ${PULL_POLICY} == "never" ]] && ! image_exists; then
    die "Image is not present locally and --pull never was selected: ${IMAGE}" "${EXIT_RUNTIME}"
  fi
}

# @description Derive an owner/repository identifier from the origin remote.
# @stdout GitHub repository identifier, or an empty line when unavailable.
github_repository() {
  local remote_url=""
  local repository=""
  remote_url="$(git -C "${WORKSPACE}" remote get-url origin 2>/dev/null || true)"

  case "${remote_url}" in
  git@github.com:*) repository="${remote_url#git@github.com:}" ;;
  ssh://git@github.com/*) repository="${remote_url#ssh://git@github.com/}" ;;
  https://github.com/*) repository="${remote_url#https://github.com/}" ;;
  http://github.com/*) repository="${remote_url#http://github.com/}" ;;
  esac
  repository="${repository%.git}"
  printf "%s\n" "${repository}"
}

# @description Derive the current symbolic Git branch as a full GitHub ref.
# @stdout Full branch ref, or an empty line for detached worktrees.
github_ref() {
  local branch=""
  branch="$(git -C "${WORKSPACE}" symbolic-ref --quiet --short HEAD 2>/dev/null || true)"
  if [[ -n ${branch} ]]; then
    printf "refs/heads/%s\n" "${branch}"
  fi
}

# @description Append one environment assignment to the container command.
# @arg $1 string NAME=VALUE assignment.
append_env() {
  RUN_COMMAND+=("--env" "$1")
}

# @description Build the complete container command as a shell-safe array.
build_run_command() {
  local repository=""
  local ref=""
  local item=""

  RUN_COMMAND=("${RUNTIME}" "run" "--rm")

  case "${TTY_MODE}" in
  always) RUN_COMMAND+=("--interactive" "--tty") ;;
  auto)
    if [[ -t 0 && -t 1 ]]; then
      RUN_COMMAND+=("--interactive" "--tty")
    fi
    ;;
  esac

  RUN_COMMAND+=("--volume" "${WORKSPACE}:${CONTAINER_WORKSPACE}:rw")
  RUN_COMMAND+=("--workdir" "${CONTAINER_WORKSPACE}")

  if [[ ${USER_MODE} == "host" ]]; then
    command -v id >/dev/null 2>&1 || die "--user host requires the id command."
    RUN_COMMAND+=("--user" "$(id -u):$(id -g)")
  fi

  [[ -z ${PLATFORM} ]] || RUN_COMMAND+=("--platform" "${PLATFORM}")
  [[ -z ${ENV_FILE} ]] || RUN_COMMAND+=("--env-file" "${ENV_FILE}")

  if [[ ${DOCKER_SOCKET} == "true" ]]; then
    if [[ ${RUNTIME} == "docker" ]]; then
      [[ -S "/var/run/docker.sock" ]] || die "Docker socket not found: /var/run/docker.sock"
      RUN_COMMAND+=("--volume" "/var/run/docker.sock:/var/run/docker.sock:rw")
    else
      local podman_socket="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/podman/podman.sock"
      [[ -S ${podman_socket} ]] || die "Podman socket not found: ${podman_socket}"
      RUN_COMMAND+=("--volume" "${podman_socket}:/var/run/docker.sock:rw")
    fi
  fi

  for item in "${EXTRA_VOLUMES[@]}"; do
    RUN_COMMAND+=("--volume" "${item}")
  done

  append_env "GITHUB_WORKSPACE=${CONTAINER_WORKSPACE}"
  append_env "VALIDATE_ALL_CODEBASE=$([[ ${CHANGED_ONLY} == "true" ]] && printf false || printf true)"

  [[ -z ${ENABLE_DESCRIPTORS} ]] || append_env "ENABLE=${ENABLE_DESCRIPTORS}"
  [[ -z ${ENABLE_LINTERS} ]] || append_env "ENABLE_LINTERS=${ENABLE_LINTERS}"
  [[ -z ${DISABLE_DESCRIPTORS} ]] || append_env "DISABLE=${DISABLE_DESCRIPTORS}"
  [[ -z ${DISABLE_LINTERS} ]] || append_env "DISABLE_LINTERS=${DISABLE_LINTERS}"
  [[ -z ${FIX_VALUE} ]] || append_env "APPLY_FIXES=${FIX_VALUE}"

  if [[ ${REPORT_DIRECTORY} == "none" ]]; then
    append_env "REPORT_OUTPUT_FOLDER=none"
  else
    append_env "REPORT_OUTPUT_FOLDER=${CONTAINER_WORKSPACE}/${REPORT_DIRECTORY}"
  fi

  if [[ -n ${CONFIG_FILE} ]]; then
    append_env "MEGALINTER_CONFIG=${CONTAINER_WORKSPACE}/${CONFIG_FILE#"${WORKSPACE}/"}"
  fi

  repository="$(github_repository)"
  ref="$(github_ref)"
  [[ -z ${repository} ]] || append_env "GITHUB_REPOSITORY=${repository}"
  [[ -z ${ref} ]] || append_env "GITHUB_REF=${ref}"

  if [[ ${DEBUG_MODE} == "true" ]]; then
    append_env "LOG_LEVEL=DEBUG"
    append_env "PRINT_ALL_FILES=true"
  fi

  for item in "${EXTRA_ENVS[@]}"; do
    append_env "${item}"
  done
  for item in "${RUNTIME_ARGS[@]}"; do
    RUN_COMMAND+=("${item}")
  done

  RUN_COMMAND+=("${IMAGE}")
}

# @description Render one command argument using conservative POSIX quoting.
# @arg $1 string Argument value.
# @stdout Shell-quoted argument.
shell_quote() {
  local value="$1"
  if [[ ${value} =~ ^[A-Za-z0-9_./:@%+=,-]+$ ]]; then
    printf "%s" "${value}"
  else
    printf "'"
    printf "%s" "${value}" | sed "s/'/'\\\\''/g"
    printf "'"
  fi
}

# @description Print the generated container command with environment values redacted.
# @stdout Multiline shell command safe to share in diagnostics.
print_redacted_command() {
  local index=0
  local argument=""
  local redact_next="false"

  while [[ ${index} -lt ${#RUN_COMMAND[@]} ]]; do
    argument="${RUN_COMMAND[$index]}"
    if [[ ${redact_next} == "true" ]]; then
      if [[ ${argument} == *=* ]]; then
        shell_quote "${argument%%=*}=<redacted>"
      else
        shell_quote "<redacted>"
      fi
      redact_next="false"
    else
      shell_quote "${argument}"
      if [[ ${argument} == "--env" || ${argument} == "--env-file" ]]; then
        redact_next="true"
      fi
    fi
    index=$((index + 1))
    if [[ ${index} -lt ${#RUN_COMMAND[@]} ]]; then
      printf ' %b' '\134'
      printf "\n  "
    else
      printf "\n"
    fi
  done
}

# @description Report runtime, image, configuration, and report readiness.
run_doctor() {
  local failures=0

  printf "MegaLinter wrapper doctor\n\n"
  printf "%-14s %s\n" "workspace" "${WORKSPACE}"
  printf "%-14s %s\n" "config" "${CONFIG_FILE:-not found (MegaLinter defaults apply)}"
  printf "%-14s %s\n" "runtime" "${RUNTIME}"
  printf "%-14s %s\n" "image" "${IMAGE}"
  printf "%-14s %s\n" "reports" "${REPORT_DIRECTORY}"

  if runtime_ready; then
    printf "%-14s %s\n" "runtime state" "ready"
  else
    printf "%-14s %s\n" "runtime state" "unavailable"
    failures=$((failures + 1))
  fi

  if image_exists; then
    printf "%-14s %s\n" "image state" "available locally"
  else
    printf "%-14s %s\n" "image state" "not present locally (pull policy: ${PULL_POLICY})"
    if [[ ${PULL_POLICY} == "never" ]]; then
      failures=$((failures + 1))
    fi
  fi

  if [[ ${failures} -ne 0 ]]; then
    return "${EXIT_RUNTIME}"
  fi
}

# @description Execute MegaLinter or print its dry-run command.
run_megalinter() {
  local exit_code=0

  if [[ ${DRY_RUN} == "true" ]]; then
    printf "# Environment values are redacted.\n"
    print_redacted_command
    return 0
  fi

  runtime_ready || die "${RUNTIME} is installed but its service is not ready." "${EXIT_RUNTIME}"
  prepare_image

  log_info "Workspace: ${WORKSPACE}"
  log_info "Image: ${IMAGE}"
  log_info "Config: ${CONFIG_FILE:-MegaLinter defaults}"
  log_info "Reports: ${REPORT_DIRECTORY}"
  log_info "Running MegaLinter"

  set +o errexit
  "${RUN_COMMAND[@]}"
  exit_code=$?
  set -o errexit

  if [[ ${exit_code} -eq 0 ]]; then
    log_info "MegaLinter completed successfully."
  else
    log_error "MegaLinter exited with status ${exit_code}."
  fi
  return "${exit_code}"
}

# @description Orchestrate argument parsing, validation, and MegaLinter execution.
# @arg $@ string Wrapper command-line arguments.
main() {
  parse_arguments "$@"
  resolve_workspace
  resolve_config
  validate_report_directory
  validate_options
  select_runtime
  resolve_image
  build_run_command

  log_debug "Workspace resolved to ${WORKSPACE}"
  log_debug "Container runtime resolved to ${RUNTIME}"
  log_debug "MegaLinter image resolved to ${IMAGE}"

  if [[ ${DOCTOR_MODE} == "true" ]]; then
    run_doctor
    return $?
  fi

  run_megalinter
}

if [[ ${BASH_SOURCE[0]} == "${0}" ]]; then
  main "$@"
fi
