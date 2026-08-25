# Portability rules

Egolint's native portability pack detects repository properties that routinely
break when a checkout moves between Linux, macOS, Windows, a Dev Container, and
GitHub-hosted runners. “Windows-compatible shell” means Git-for-Windows Bash,
PowerShell, or another explicitly selected shell; it does not mean that POSIX
scripts are interpreted by `cmd.exe`.

The machine-readable source of truth is
`.config/rules/portability.toml`. Every finding contains the stable rule ID,
severity, normalized source path and position where available, policy owner,
catalog path, and deterministic fingerprint.

| Rule                    | Severity | Surface                                                           |
| ----------------------- | -------- | ----------------------------------------------------------------- |
| `EGO-PORT-CASE-001`     | Error    | ASCII case-folding collisions in repository paths                 |
| `EGO-PORT-PATH-001`     | Error    | Windows device names, reserved characters, and trailing dot/space |
| `EGO-PORT-EOL-001`      | Error    | Mixed LF/CRLF or lone carriage returns                            |
| `EGO-PORT-EOL-002`      | Error    | CRLF in portable automation files                                 |
| `EGO-PORT-EXEC-001`     | Error    | Tracked interpreter shebangs without Git mode `100755`            |
| `EGO-PORT-HOME-001`     | Warning  | Literal workstation home directories in automation                |
| `EGO-PORT-CMD-001`      | Warning  | Reviewed GNU/BSD command-form differences                         |
| `EGO-PORT-WORKFLOW-001` | Warning  | Multi-OS GitHub Actions steps without an explicit shell           |

The path and executable checks use Git's NUL-delimited inventory and index mode,
not newline parsing or the current host's filesystem permission bits. This keeps
results deterministic on Windows. Executable-mode enforcement requires the
first line to begin with `#!` followed by an absolute interpreter path (with
optional horizontal whitespace), so language syntax such as Rust's `#![...]`
inner attributes is not classified as a script. The case-collision check
deliberately uses ASCII folding: it catches the common Git/Windows failure
without pretending to implement every filesystem's locale- and Unicode-specific
comparison behavior.

The GNU/BSD rule is advisory because a lexical command match cannot prove the
surrounding runtime guard. Use an expiring suppression when a script performs an
explicit platform branch or intentionally targets only one operating system.

Authoritative rationale is linked per rule in the catalog, principally Git's
line-ending documentation, Microsoft's Win32 naming rules, POSIX utility
specifications, and GitHub Actions' shell table. These are living standards
references; the local policy catalog—not an external webpage—is the versioned
Egolint selection.
