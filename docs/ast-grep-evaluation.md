# ast-grep structural policy evaluation

Issue [#37](https://github.com/egohygiene/egolint/issues/37). Decision: **adapt as
an optional, pinned, advisory host adapter**. Keep it outside the default full
engine image. The native CLI normalizes its evidence; it never uses ast-grep's
rewrite path or loads repository-owned rule/config/parser code.

## Selected immutable inputs and maintenance

- ast-grep **0.51.1**, upstream release published 2026-11-02.
- Source commit `5a204b1ef3facef94de125d990dddc7cf90011c0` from the verified
  [0.51.1 tag](https://github.com/ast-grep/ast-grep/tree/5a204b1ef3facef94de125d990dddc7cf90011c0).
- Upstream [MIT license](https://github.com/ast-grep/ast-grep/blob/5a204b1ef3facef94de125d990dddc7cf90011c0/LICENSE).
- npm `@ast-grep/cli@0.51.1` integrity:
  `sha512-xdX9+jHTZLwpYtHFRmK4Zzoe0UPmZrn037wwCHKBqaFJPC12xiGTwrZEZDMbBwRwmsGYcG1yHsb7yvEFU8EzNw==`.
- The exercised Linux x86-64 prebuilt executable is 55,722,000 bytes, SHA-256
  `c4c1970bf0da5a4a81b37b682b7feb1f00d623e23ec40bbf1546fe9bf65c7306`.

The observed release list includes 0.50.0 (October 9), 0.51.0 (October 15) and
0.51.1 (November 2), establishing recent maintenance without assuming future
support. Early compatibility controls used 0.40.5; the selected adapter requires
0.51.1 exactly and treats a mismatched version as configuration failure.

The installation evaluation used the exact npm package and its verified
registry integrity. Lifecycle scripts were disabled; the platform package's
prebuilt executable was invoked directly. A developer can install that exact
version with their normal package tooling. No binary or dependency tree is
vendored into EgoLint, and no image size or supply-chain expansion is imposed
on default consumers. Platform binaries have independent hashes; the Linux
digest is not asserted for another platform.

## Rules and ownership

ast-grep uses tree-sitter parsers, metavariable patterns and relational
composition. Its wider language support is useful, but this adapter qualifies
only the selected JavaScript, TypeScript, Python and Rust forms. The embedded
versioned rules have stable IDs, warning severity and explicit review messages.

| Rule                   | Structural requirement                                       | Negative controls / limits                                                                                                          |
| ---------------------- | ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| `EGO-AST-JS-EVAL-001`  | Direct `eval(argument)` call                                 | Comments, strings and member calls do not match; binding/shadowing is not resolved.                                                 |
| `EGO-AST-TS-EVAL-001`  | The equivalent TypeScript call                               | Same syntactic boundary; no type/data-flow claim.                                                                                   |
| `EGO-AST-PY-SHELL-001` | `subprocess.run` with a direct `shell=True` keyword argument | `shell=False`, quoted examples and a nested helper's `shell=True` do not match. Aliased APIs and computed values are outside scope. |
| `EGO-AST-RS-EXIT-001`  | Direct `std::process::exit` call                             | Comments/strings do not match. CLI exits may be intentional; aliases and reusable-code context are not inferred.                    |

These are focused review prompts. They overlap syntactically with some Semgrep
or language-linter rules, but do not replace their security, data-flow, compiler
or type ownership. Existing Semgrep policy remains unchanged. EgoLint-native
Rust rules continue to own repository contracts, configuration, portability,
provenance and enforcement. A future rule needs an explicit owner, version,
positive/hostile controls and overlap decision; adding a parser is insufficient.

## Execution and evidence

Use `egolint validate --ast-grep`, or declare `AST_GREP` under capabilities.
Source applicability is checked before invoking the runtime. The installed
executable's exact version is checked. Execution uses direct argv, embedded
rules, explicit regular-file inputs and a separate temporary working directory;
repository `sgconfig`, parser plugins and fix directives are not loaded.
There is a shared 30-second deadline, bounded output, file batches, source-count
and finding limits. No network or runtime installation is requested.

The normalizer allowlists known rule IDs and workspace-relative spans, supplies
owned messages/remediation and retains no matched text, snippets, metavariable
values or raw stderr. JSON/SARIF keep rule/tool, file, location, severity and
canonical identity. Existing explicit suppression handling remains available.
Unsupported/no-source repositories skip deterministically. Missing or
incompatible runtimes remain visible failures when execution was requested;
observe mode does not waive them. `fix --ast-grep` is rejected.

## Reproduce the evaluation

```sh
cargo build --locked
python3 scripts/benchmark_ast_grep.py --binary target/debug/egolint
```

The driver creates temporary synthetic Git repositories and invokes the actual
native adapter. It never executes fixture source. Three datasets contain 8,
256 and 1,024 source files; each includes positive and negative examples for
all four rules. It verifies normalized findings and advisory ownership and
records adapter/end-to-end timing and executable digest. The batch ledger
records the final measured results.

These controls do not establish general precision. Shadowed identifiers,
intentional CLI exits, broader suppression behavior, non-ASCII column semantics,
large-repository stress and non-Linux/native-image qualification remain review
work. The adapter therefore stays opt-in and advisory. A tool/version upgrade
must rerun these controls and inspect output/precision changes before updating
the exact pin. Removing the declaration is the immediate rollout rollback;
reverting the source change restores the prior registry. No publication or
cross-platform acceptance is claimed.
