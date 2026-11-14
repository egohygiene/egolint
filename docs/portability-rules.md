# Portability rules

Egolint's native portability pack detects repository properties that routinely break when a checkout
moves between Linux, macOS, Windows, a Dev Container, and GitHub-hosted runners. “Windows-compatible
shell” means Git-for-Windows Bash, PowerShell, or another explicitly selected shell; it does not
mean that POSIX scripts are interpreted by `cmd.exe`.

The machine-readable source of truth is `.config/rules/portability.toml`. Every finding contains the
stable rule ID, severity, normalized source path and position where available, policy owner, catalog
path, and deterministic fingerprint.

| Rule                    | Severity | Surface                                                           |
| ----------------------- | -------- | ----------------------------------------------------------------- |
| `EGO-PORT-CASE-001`     | Error    | ASCII case-folding collisions in paths and inferred directories   |
| `EGO-PORT-PATH-001`     | Error    | Windows device names, reserved characters, and trailing dot/space |
| `EGO-PORT-EOL-001`      | Error    | Mixed LF/CRLF or lone carriage returns                            |
| `EGO-PORT-EOL-002`      | Error    | CRLF in portable automation files                                 |
| `EGO-PORT-EXEC-001`     | Error    | Tracked interpreter shebangs without Git mode `100755`            |
| `EGO-PORT-HOME-001`     | Warning  | Literal workstation home directories in automation                |
| `EGO-PORT-CMD-001`      | Warning  | Reviewed GNU/BSD command-form differences                         |
| `EGO-PORT-WORKFLOW-001` | Warning  | Multi-OS GitHub Actions steps without an explicit shell           |

The path and executable checks use Git's NUL-delimited inventory and index mode, not newline parsing
or the current host's filesystem permission bits. This keeps results deterministic on Windows.
Executable-mode enforcement requires the first line to begin with `#!` followed by an absolute
interpreter path (with optional horizontal whitespace), so language syntax such as Rust's `#![...]`
inner attributes is not classified as a script. The case-collision check deliberately uses ASCII
folding: it catches the common Git/Windows failure without pretending to implement every
filesystem's locale- and Unicode-specific comparison behavior.

The topology includes inferred directory prefixes: a file `Readme` conflicts
with `README/page.md`, and `Docs/a.md` conflicts with `docs/b.md`. Each folded
node reports its distinct component spellings once; identical descendant names
do not repeat an ancestor's finding. Consistently cased shared directories remain
valid. Native Git/CLI regression cases exercise a case-sensitive Linux checkout;
they do not establish macOS or Windows checkout compatibility.

The GNU/BSD rule is advisory because a lexical command match cannot prove the surrounding runtime
guard. Use an expiring suppression when a script performs an explicit platform branch or
intentionally targets only one operating system.

The workflow shell rule expands static matrix axes, applies matching `exclude`
rows first, then processes `include` in declaration order. Includes can add or
replace extra metadata on original combinations without overwriting an original
axis value; unmatched includes create additional combinations and later includes
do not merge into those added rows. Include-only matrices have no phantom base
combination. Partial excludes match all named values, so excluding one version
does not remove other versions on the same operating system.

Supported `runs-on` forms are one exact scalar expression such as
`${{ matrix.os }}` or `${{ matrix.runner.os }}` over static object-valued axes.
Property names are matched completely: `matrix.os-version` is not `matrix.os`.
Conventional literal runner labels determine Windows, macOS and Linux families;
these are declared labels, not actual runner probes. An explicit step shell or
job/workflow `defaults.run.shell` suppresses the omitted-shell advisory without
proving that the command or selected shell works on each runner.

Expressions in matrix data, `fromJSON`, operators, interpolated/bracket selectors,
label arrays/groups, unrecognized runner families and unsupported matrix shapes
remain unresolved. Expansion is bounded to 256 combinations, 4096 static value
nodes and depth 16. Unresolved or over-budget input emits no shell advisory;
that means unknown diagnostic coverage, never demonstrated portability. No
GitHub expression, workflow command or provider operation is executed.
Step positions are bound only when the plain block source agrees with every
parsed run step. Matrix/default/env metadata is not a run step; ambiguous literal
script keys, flow/quoted/aliased layouts or incomplete source correspondence
retain the workflow filename with unknown line/column instead of guessing.
Native Git/CLI fixtures qualify static findings and positions on Linux only;
they do not run workflow steps or establish hosted/macOS/Windows execution.

Authoritative rationale is linked per rule in the catalog, principally Git's line-ending
documentation, Microsoft's Win32 naming rules, POSIX utility specifications, and GitHub Actions'
shell table. These are living standards references; the local policy catalog—not an external
webpage—is the versioned Egolint selection.
