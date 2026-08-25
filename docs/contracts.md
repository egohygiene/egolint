# Machine-readable contracts

Egolint owns versioned contracts for profiles, findings, suppressions, evidence,
per-tool results, execution plans, run reports, the repository-contract
validation envelope, and Repository Intelligence semantic policy. The checked-in JSON Schemas in
[`schemas/`](../schemas/) are generated from the Rust types used by the CLI; CI
rejects schema drift.

All current contracts use version `1`, and every generated schema constrains its
version field to exactly `1`. Contract changes must update the Rust type,
checked-in schema, compatibility fixtures, documentation, and changelog in one
review.

JSON Schema validates contract structure, required fields, enums, and exact
versions. Rust validation, invoked by the CLI before persistence, additionally
enforces semantic invariants such as normalized relative paths, real Gregorian
dates, bounded text, digests, and cross-field report consistency.

## Profile scope

The fast profile records `changed_files_with_repository_policy`: MegaLinter
targets changed files while Egolint's native portability and repository-policy
checks inspect the complete repository inventory. `changed_files` remains a
decodable version-1 value for compatibility with reports written before native
repository checks were added. Holistic, security, and dependency-debt profiles
record `complete_repository`.

## Ownership and evidence

A finding carries a stable tool/rule identity and structured ownership:

- `owner` identifies the repository, team, or delegated system accountable for
  the rule.
- `policy_source` identifies the policy decision that selected it.
- `configuration_path` identifies the optional workspace-relative rule file.
- `evidence` contains only reviewed, workspace-relative references. Raw adapter
  environments and arbitrary absolute host paths are not evidence contracts.

The generated MegaLinter tool matrix applies the same model to all 124 pinned
tools. Selection reason, enforcement, ownership, configuration, fixtures, and
expected runtime report path remain distinct fields.

## Findings and suppressions

Finding severities are `info`, `warning`, `error`, and `critical`. Unsuppressed
`error` and `critical` findings participate in the blocking run status;
informational and warning findings remain visible without independently failing
the run. A blocking tool result may still fail the run when its adapter reports
findings that have not yet been normalized individually.

A suppression always records a stable identifier, rule selector, owner,
justification, and real Gregorian expiry date. It may narrow the selector with a
workspace-relative path or finding fingerprint. Applied, unmatched, expired,
and invalid states remain distinguishable. Rule engines evaluate dates and
matches; the base contract does not silently discard expired or unmatched
entries. The portability suppression engine also requires reviewed evidence;
schema-only structural validation does not establish that policy requirement.

## Honest report completeness

`RunReport.completeness` prevents an empty normalized array from being mistaken
for complete coverage:

- `adapter_exit_only` means only the wrapped process outcome is known.
- `partial` means some adapter details were normalized but coverage is
  incomplete.
- `normalized` means every available adapter result was normalized.

The summary counts only objects actually present in `tool_results`, `findings`,
and `suppressions`. `RunReport::from_plan` uses `adapter_exit_only` when only a
process outcome is known. Completed CLI commands replace that placeholder
before persistence: `validate` and `fix` emit partial native detail, while
`lint` emits partial or normalized detail according to available adapter
coverage. Integrations can use the same validated report API as normalizers
become available.

## Commands

```sh
egolint validate --profile "fast"
egolint lint --profile "fast"
egolint explain --format "json"
egolint doctor --profile "holistic"
egolint schema finding
egolint schema repository-contract
egolint schema repository-intelligence
```

`validate` resolves every configuration layer, evaluates native portability
policy plus requested repository contracts, Repository Intelligence sources,
and suppressions, and writes
`.reports/egolint/run.json` and canonical SARIF without starting a container.
`plan` prints the redacted execution plan. `doctor` additionally requires and
probes Docker or Podman before printing that plan. `check` is a compatibility
alias for `lint`; `config explain` remains a compatibility form of `explain`.

## Empathy compatibility

The first executable compatibility fixture is
[`tests/fixtures/compatibility/empathy-v1/`](../tests/fixtures/compatibility/empathy-v1/).
It pins the extracted source to `egohygiene/empathy` commit
`560aff8430c2f170dadae9161a4603a71c41acbf`, verifies holistic profile resolution
and the 124/12/105 catalog/fast/holistic inventory, and round-trips sanitized
finding, suppression, and report examples. The fixture intentionally excludes
Empathy's generated MegaLinter report because that artifact contains cached
process environments, absolute paths, and unrelated repository state.
