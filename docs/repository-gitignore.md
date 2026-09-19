# Repository gitignore validation

`egolint gitignore` validates the declared composition and effective Git behavior of a consumer.
It requires Git, local reviewed source files, and an Empathy composition plan. It runs without
Docker, a network fetch, or a MegaLinter repair. Required-file presence remains a separate check
under [repository contracts](repository-contracts.md).

```sh
egolint --workspace /path/to/consumer gitignore \
  --policy foundation/gitignore-policy.toml \
  --evaluation-date 2026-09-19
```

The policy path is relative to the worktree root. The explicit date makes exception expiry
repeatable; CI callers must supply their current evaluation date. Do not reuse a historical date
to extend an exception. Configuration errors return the existing configuration-error exit code
before evaluation. Once the request is valid, missing Git, malformed provider/composition inputs,
drift, and incomplete inventory produce machine-readable findings and a nonzero result.

## Ownership and accepted inputs

Empathy owns the baseline, overlays, profile selection, and composition format. Holon owns
materialization and recovery. EgoLint validates those inputs and the consumer without changing
either. This adapter initially supports exactly:

- `empathy/repository-foundation@1.1.0` and `empathy.gitignore/v1`;
- Empathy commit `b44f798bb49259f9f48416b4ffebde1103e135c0`;
- the reviewed catalog and fragment digests in `.config/rules/repository-gitignore.v1.json`.

Export `foundation/catalog.json` and every `foundation/ignore/` fragment from that commit into a
local directory such as `vendor/empathy`. Supply the reviewed JSON output of Empathy's
`plan-gitignore` and the explicit consumer selection below. Generate/check the plan with Empathy
before adopting it through Holon. EgoLint neither executes source-side code nor resolves a new
foundation manifest. It validates the selected profiles and dependencies, scope inventory,
repository ownership, exact local text, ordered layers, framing, and content digests. The full
foundation manifest resolution remains Empathy's `check-gitignore-plan` responsibility; the
resolved-manifest digest retained by the composition is provenance, not an additional proof of
that resolution.

A source-pin update requires a reviewed adapter/catalog and compatibility-fixture change. A
consumer cannot replace the supported upstream digest merely by changing its own request.
The runtime contains probe paths and source identities, not another maintained ignore-rule corpus.
The exact upstream fragments under `tests/fixtures/` are offline compatibility fixtures.

## Consumer policy

The Filament example uses the accepted root composition unchanged:

```toml
schema-version = 1
id = "filament-gitignore"
repository = "egohygiene/filament"
source-revision = "b44f798bb49259f9f48416b4ffebde1103e135c0"
source-root = "vendor/empathy"
composition-path = "foundation/gitignore-plan.json"
composition-sha256 = "aa2eee88be4b0922be640c30b0ed22ea381d25339a11e524221bd6620eb996ab"
profiles = ["universal"]

[[scopes]]
root = "."
overlays = []
local-additions = ""
```

`source-root` may be `.` for an in-place reviewed source export. All input paths must be normalized,
relative, literal paths outside Git metadata and the writable `.reports/egolint/` boundary. Inputs must be regular files; symlinks are rejected.
Unknown policy fields are rejected. Scopes must include the root and follow the composition's
order. Profiles are the complete resolved list, including `universal`. Overlay order and local
text must exactly match the composition. Empty local text is valid; nonempty local text must be
LF-terminated and have explicit consumer behavior probes in its scope.

Digests use SHA-256. Fragment and consumer-file digests cover exact UTF-8 bytes. The composition,
source catalog, and parsed policy use compact JSON with recursively sorted object keys, UTF-8
characters unescaped, and array order preserved. Whitespace changes to a JSON document therefore
do not alter its semantic digest. TOML formatting does not alter the policy digest.

`tests/fixtures/repository-gitignore/scoped-rust/` gives an executable example selecting the
`rust-build` overlay only at `crates/widget`, preserving `/scratch/` and a visible
`target/keep.txt`. Consumer probes assert literal path behavior:

```toml
[[probes]]
path = "crates/widget/target/keep.txt"
ignored = false
```

These probes supplement the pinned universal vectors. Expected behavior normally comes from the
validated composition in a reference Git repository; an explicit probe asserts the consumer's
intended outcome, including cases where its declared rules contain an unreachable negation.
Repository-owned local text and `override = "preserve"` never exempt content drift. New output
ownership must first be reconciled in the owning composition.

## Evidence layers and outcomes

The command writes the standard `.reports/egolint/run.json`, canonical `egolint.sarif`, and focused
`repository-gitignore.json`. Native findings use `EGOLINT_REPOSITORY_GITIGNORE` and stable
`EGO-IGNORE-*-001` rule identities. Diagnostics include remediation; behavior evidence retains the
path, scope, expected/actual visibility, winning policy/line/pattern, tracked state, and applied
exception reference. The report binds the source pin, policy digest, and composition digest.

| Check | What a pass establishes |
| --- | --- |
| `source_integrity` | Supported immutable catalog and every source fragment match their pins. |
| `composition` | The plan exactly represents the declared supported selection and ordered text. |
| `presence` | Every declared `.gitignore` exists as a readable regular policy file. |
| `content` | Consumer bytes match the reviewed composition at every declared scope. |
| `effective_behavior` | Actual Git visibility agrees for all evaluated paths or exact reviewed exceptions. |
| `nested_policy` | Every inherited policy has current explicit inventory review. |
| `tracked_files` | No indexed protected path lacks an exact reviewed tracked-file exception. |
| `coverage` | The bounded inventory is complete and stable during evaluation. |

Each layer reports `passed`, `failed`, `incomplete`, `unavailable`, or `not_evaluated`. Overall
`valid` requires every layer to pass. `valid_with_exceptions` retains informational findings and
all exception evidence. Any observed violation yields `invalid`; unavailable/incomplete evidence
without a known violation yields `incomplete`. Both latter states produce blocking findings, so
an absent Git executable or an undeclared harmless nested policy cannot become an empty clean run.
The enclosing run is `partial`: it covers this native capability, not the full lint tool inventory.

## Nested policies and exceptions

Every discovered `.gitignore`, including policies beneath ignored directories, is inventoried.
An undeclared policy makes the result incomplete. Declare it as a composition scope or add a
`[[nested-policies]]` record with `path`, exact `sha256`, `owner`, `reason`, and HTTPS `approval`.
This record establishes ownership/disposition only. Semantic differences still fail.

A justified deviation uses a separate `[[exceptions]]` record with one literal `path`, its winning
`policy-path` and `policy-sha256`, `owner`, `reason`, HTTPS `approval`, and `expires-on`. Set
`ignored` to the approved actual visibility and/or `allow-tracked = true` for an approved already
indexed path. Wildcards, blanket scope exemptions, expired/stale reviews, and unused exceptions
are rejected or fail evaluation. Changing the winning policy invalidates the exception. Approval
URLs are locally supplied provenance, not a claim that EgoLint fetched or authorized the review.

Empathy's inherited devcontainer, Mantle, React-template, and unanchored nested target policies are
negative fixtures. Their inventory does not make them conformant. Canonical source/disposition
reconciliation remains [Empathy #92](https://github.com/egohygiene/empathy/issues/92), coordinated
with [#79](https://github.com/egohygiene/empathy/issues/79).

## Read-only and coverage boundary

Only policy/input text, pathname metadata, and Git index metadata are read. The library writes
nothing to the consumer; the CLI writes only its report directory. Ordinary file contents are not
copied into fixtures or reports. Two disposable repositories contain validated expected policies
and actual policy snapshots, directory topology, and empty probe placeholders. Real
`git check-ignore --no-index --verbose --non-matching --stdin -z` supplies winning-rule evidence.

Inherited Git environment overrides, system/global excludes, templates, and fsmonitor are isolated.
Consumer `.git/info/exclude` is intentionally outside this portable repository-policy contract.
Already tracked paths are queried separately without optional locks; ignore rules do not protect
them. This is neither secret scanning nor proof that an approved tracked payload is safe.

The inventory is limited to 50,000 entries/probes, 256 policies, 1 MiB per input, and 8 MiB total
ignore text. Each Git invocation has a 30-second/32-MiB evidence budget. Symlinks, submodules,
nested repositories, unmerged indexes, unsupported names, unreadable paths, incompatible probe
file/directory topology, or exhausted budgets prevent complete coverage. Git and filesystem
coverage include the working tree and index, not history. No fetch, staging, untracking, cleanup,
or rule rewrite occurs. Other writers must remain quiescent; before/after checks detect policy,
pathname, and tracked-path changes but are not an atomic filesystem snapshot.

The finite upstream vectors are expanded at each declared or discovered policy scope and combined
with actual file names, tracked paths, consumer probes, and exception paths. This catches known
nested weakening without claiming proof over every future possible pathname. Add reviewed probes
for repository-specific cases. Consumers should inspect `checks` and `limitations`, never infer
semantic safety from file existence or an empty findings array alone.

## Integration sequence

[Issue #61](https://github.com/egohygiene/egolint/issues/61) delivers the standalone native command,
library types, schemas, and evidence. It is deliberately separate from general `validate`/`lint`
inventory, which reads ordinary source payloads for other rules. Future Relay integration can
invoke this command and consume the standard reports after the capability is accepted and pinned.

After maintainer merge, Empathy #92 proves golden/Filament consumers through accepted Holon and
EgoLint interfaces. Relay #5/#49 retain shared CI/release prerequisites, and Pace #30 retains fleet
adoption. The [master epic](https://github.com/egohygiene/.github/issues/32) owns gitignore closeout
and queues Empathy #91 for `.gitattributes`. Direct MegaLinter repairs remain deferred.
