# Machine-readable contracts

Egolint owns versioned contracts for profiles, findings, suppressions, evidence, per-tool results,
execution plans, run reports, the repository-contract validation envelope, Repository Continuity,
and Repository Intelligence semantic policy. The checked-in JSON Schemas in
[`schemas/`](../schemas/) are generated from the Rust types used by the CLI; CI rejects schema
drift.

Consumer delivery is separately bound by
[`integrations/contract.json`](../integrations/contract.json). That contract versions the GitHub
Action, MegaLinter adapter image, pre-commit hook, VS Code task, canonical report paths, and fix
authority as one release-compatible surface. The deterministic integration archive adds a generated
manifest with the Cargo package version and SHA-256 for every bundled file.

All current contracts use version `1`, and every generated schema constrains its version field to
exactly `1`. Contract changes must update the Rust type, checked-in schema, compatibility fixtures,
documentation, and changelog in one review.

JSON Schema validates contract structure, required fields, enums, and exact versions. Rust
validation, invoked by the CLI before persistence, additionally enforces semantic invariants such as
normalized relative paths, real Gregorian dates, bounded text, digests, and cross-field report
consistency.

## Profile scope

The fast profile records `changed_files_with_repository_policy`: MegaLinter targets changed files
while Egolint's native portability and repository-policy checks inspect the complete repository
inventory. `changed_files` remains a decodable version-1 value for compatibility with reports
written before native repository checks were added. Holistic, security, and dependency-debt profiles
record `complete_repository`.

## Ownership and evidence

A finding carries a stable tool/rule identity and structured ownership:

- `owner` identifies the repository, team, or delegated system accountable for the rule.
- `policy_source` identifies the policy decision that selected it.
- `configuration_path` identifies the optional workspace-relative rule file.
- `evidence` contains only reviewed, workspace-relative references. Raw adapter environments and
  arbitrary absolute host paths are not evidence contracts.

The generated MegaLinter tool matrix applies the same model to all 124 pinned tools. Selection
reason, enforcement, ownership, configuration, fixtures, and expected runtime report path remain
distinct fields.

## Findings and suppressions

Finding severities are `info`, `warning`, `error`, and `critical`. Unsuppressed `error` and
`critical` findings participate in the blocking run status; informational and warning findings
remain visible without independently failing the run. A blocking tool result may still fail the run
when its adapter reports findings that have not yet been normalized individually.

A suppression always records a stable identifier, rule selector, owner, justification, and real
Gregorian expiry date. It may narrow the selector with a workspace-relative path or finding
fingerprint. Applied, unmatched, expired, and invalid states remain distinguishable. Rule engines
evaluate dates and matches; the base contract does not silently discard expired or unmatched
entries. The portability suppression engine also requires reviewed evidence; schema-only structural
validation does not establish that policy requirement.

## Honest report completeness

`RunReport.completeness` prevents an empty normalized array from being mistaken for complete
coverage:

- `adapter_exit_only` means only the wrapped process outcome is known.
- `partial` means some adapter details were normalized but coverage is incomplete.
- `normalized` means every available adapter result was normalized.

The summary counts only objects actually present in `tool_results`, `findings`, and `suppressions`.
`RunReport::from_plan` uses `adapter_exit_only` when only a process outcome is known. Completed CLI
commands replace that placeholder before persistence: `validate` and `fix` emit partial native
detail, while `lint` emits partial or normalized detail according to available adapter coverage.
Integrations can use the same validated report API as normalizers become available.

## Commands

```sh
egolint validate --profile "fast"
egolint lint --profile "fast"
egolint explain --format "json"
egolint doctor --profile "holistic"
egolint schema finding
egolint schema repository-contract
egolint schema repository-intelligence
egolint schema repository-intelligence-report
egolint schema repository-presentation
egolint schema repository-presentation-report
egolint schema repository-continuity
egolint schema repository-continuity-report
egolint schema repository-gitignore
egolint schema repository-gitignore-report
egolint schema repository-release-report
```

`validate` resolves every configuration layer, evaluates native portability policy plus requested
repository contracts, explicit-base/head repository continuity, Repository Intelligence sources,
repository-presentation structure and evidence, and suppressions. It writes
`.reports/egolint/run.json`, canonical SARIF, and any selected focused report without starting a
container. See [repository continuity validation](repository-continuity.md) for the evidence-layer
and rollout semantics. `plan` prints the redacted execution plan. `doctor` additionally requires
and probes Docker or Podman before printing that plan. `check` is a compatibility alias for `lint`;
`config explain` remains a compatibility form of `explain`.

## Empathy compatibility

The first executable compatibility fixture is
[`tests/fixtures/compatibility/empathy-v1/`](../tests/fixtures/compatibility/empathy-v1/). It pins
the extracted source to `egohygiene/empathy` commit `560aff8430c2f170dadae9161a4603a71c41acbf`,
verifies holistic profile resolution and the 124/12/105 catalog/fast/holistic inventory, and
round-trips sanitized finding, suppression, and report examples. The fixture intentionally excludes
Empathy's generated MegaLinter report because that artifact contains cached process environments,
absolute paths, and unrelated repository state.

## Layered gitignore evidence

The focused `egolint gitignore` command emits standard findings, tool results, run JSON, and SARIF,
plus `egolint.repository-gitignore-report/v1` evidence. Its closed policy and report schemas are
`repository-gitignore.schema.json` and `repository-gitignore-report.schema.json`. Presence, source
integrity, exact content, actual Git behavior, nested inventory, tracked files, and coverage are
independent checks. Unknown evidence is blocking; accepted exceptions remain visible. See
[repository gitignore validation](repository-gitignore.md) for invocation, ownership, and limits.

## Repository release evidence

Repository release conformance is derived from two separately owned inputs: Aether defines the
normative declaration schema, and Hygiene defines applicability by repository profile and
lifecycle. EgoLint vendors exact reviewed revisions and verifies their bytes before using them.
See [repository release validation](repository-release-validation.md) for the ownership boundary,
accepted revisions, and staged implementation plan.

Repository-release applicability is a universal native capability for `lint`, `validate`, and
`fix`. It discovers `.egohygiene/release.json` without a network request and always writes the
closed `egolint.repository-release-report/v1` contract to
`.reports/egolint/repository-release.json`. Missing declarations remain `unavailable`; malformed or
unsupported declarations are `invalid`; incubating rollout is `advisory`; explicitly delegated
evidence is `external`; and an authorized planner may pass
`--release-adoption-state "not-applicable"` when release policy has no meaningful application.

Checkpoint 3 derives seven source-pinned mechanical checks from repository evidence; callers
cannot provide or inflate completion counters. Stable findings from failed, unavailable, and
external checks flow through the focused report, standard run report, tool result, suppressions,
and SARIF. The report contract permits `compliant` only when every applicable local check passes
with no external or unavailable evidence. Every state also records that network access was not
performed and external publication was not verified.
