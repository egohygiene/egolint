# Repository continuity validation

EgoLint validates repository-owned continuity checkpoints without writing them, interpreting
free-form prose as truth, or fetching sibling repositories and mutable provider state. The focused
capability composes two reviewed local inputs:

- Hygiene owns applicability, rollout, exception, and required-file policy.
- Aether owns the portable `CONTINUITY.md` schema, template, instruction block, and handoff
  semantics.
- EgoLint owns deterministic conformance diagnostics and normalized report transport.
- Each repository owns its current-state wording, redaction choices, and final review.

The current Hygiene profile is `proposed`, and the pinned Aether contract is `draft` and not
release-included. Their exact reviewed bytes are immutable, but they are not represented as released
contracts. EgoLint therefore dogfoods this capability in `observe`; moving to `ratchet` or `enforce`
before the upstream lifecycle gates are satisfied is itself a finding.

## Local policy and immutable inputs

A repository selects one `egolint.repository-continuity-validation/v1` TOML policy. It identifies
the repository's kind, lifecycle, visibility, rollout stage, exact root surfaces, optional provider
instruction projections, and reviewed exceptions. It also pins:

- the Hygiene profile ID, version, status, source repository, full source commit, source path, and
  SHA-256 digest;
- the selected Aether contract, supported semantic-version range, lifecycle, release status, source
  repository, and full source commit; and
- local regular-file copies of the Aether schema, instruction, and template whose SHA-256 digests
  match the bundled rule catalog.

Ordinary validation reads only these local projections. Updating a pin is a separate reviewed
repository change. Missing projections produce explicit incomplete or invalid evidence; EgoLint
never downloads a replacement during linting.

## Required repository surfaces

For active standard and template repositories, the validator requires exact-case root regular files
named `CONTINUITY.md` and `AGENTS.md`. Symbolic links and mis-cased paths do not satisfy the
contract. The root agent file must contain exactly one byte-matching managed Aether block from the
pinned instruction. A configured provider projection is left alone when it has no continuity
wiring; if it mentions the checkpoint, contract, or skill, it must contain one unmodified managed
block as well.

`CONTINUITY.md` must have the closed Aether v1 YAML front matter, one repository-specific title, and
the twelve required level-two sections in canonical order. The validator also checks fixed byte and
line bounds, repository/visibility consistency, stable references, resolving repository-relative
canonical sources, transition state, explicit verified/partial/unavailable wording, template
placeholders, unsafe YAML features, authority-escalation phrases, and secret-shaped values. Source
content is never copied into diagnostics or the dedicated report.

These checks prove structure and locally observable consistency. They do not prove that narrative
prose is complete or factually true.

## Explicit base/head interface

Every comparison supplies a full base commit or `unborn`, a full head commit or `working-tree`, one
disposition, and an evaluation date:

```sh
cargo run --locked -- --workspace "." validate \
  --repository-contract ".config/dogfood/repository-context.toml" \
  --repository-continuity ".config/dogfood/repository-continuity.toml" \
  --continuity-base "0123456789abcdef0123456789abcdef01234567" \
  --continuity-head "working-tree" \
  --continuity-disposition "updated" \
  --continuity-transition "pull-request" \
  --continuity-live-verification "unavailable" \
  --continuity-evaluation-date "2026-09-08"
```

The canonical Task wrapper requires the same evidence rather than guessing it:

```sh
task continuity:check \
  CONTINUITY_BASE="0123456789abcdef0123456789abcdef01234567" \
  CONTINUITY_HEAD="working-tree" \
  CONTINUITY_DISPOSITION="updated" \
  EVALUATION_DATE="2026-09-08"
```

`updated` requires a content change and front-matter base equal to the supplied comparison base.
`reviewed-no-change` requires unchanged content plus passed or limited review evidence; the prior
checkpoint base must remain locally inspectable. `exception` requires one repository-matched,
approved, unexpired Hygiene exception with owner, stable approval, trigger, and exit criteria. A
touched file alone is never proof of freshness, and no mode requires the file to name the commit
that writes itself.

The reusable Action exposes the same inputs and emits `repository-continuity-report`. Its
`continuity-live-evidence` and `continuity-parallel-heads` inputs accept comma-separated values.
Relay can instead invoke the CLI directly when it needs richer evidence assembly.

## Git history and checkout behavior

EgoLint never fetches. The invoking workflow must make every base, head, prior checkpoint base, and
parallel head it wants evaluated available in the local object database.

| History shape | Deterministic behavior |
| --- | --- |
| Linear or squash result | A supplied base that is an ancestor of the head is a linear comparison. A squash commit needs no special self-reference. |
| Merge commit | A locally available multi-parent head is reported as `merge_commit`; content is still compared directly between the supplied base and head trees. |
| Rebase | If the supplied old base is no longer an ancestor, the result is `diverged` and incomplete. The caller must supply the actual represented base. |
| Working tree | Tracked plus non-ignored untracked content is compared with the supplied base. The current `HEAD` must still descend from that base. |
| Shallow clone | `shallow_repository` is recorded. A missing base is `unavailable`; the report never infers unchanged or silently deepens the checkout. |
| Unborn repository | An explicit `unborn` base is compared with the current tree without inventing a commit. |
| Parallel candidates | Each explicitly supplied head must exist locally and descend from the same base. If both candidates change the checkpoint, manual semantic reconciliation is required. |

For reusable CI, use a full checkout or fetch exact reviewed refs before invoking EgoLint. Do not
give the validator network credentials merely to conceal missing local evidence.

## Pull-request and post-merge transitions

`pull-request` accepts candidate-safe states such as `in-progress`, `ready-for-review`, or
`review-reference-recorded`. It rejects a candidate that already claims post-merge reconciliation.
The optional candidate revision may be null, avoiding a self-referential commit requirement.

`post-merge` requires `post-merge-reconciliation` or `no-active-change` and rejects stale `open` or
`draft` pull-request state. Local Git can establish topology and content, but a merge or mutable
provider-state claim still requires a separate authorized adapter. Pass
`--continuity-live-verification "verified"` only with at least one stable evidence URL; otherwise
record live state as partial or unavailable and keep the external evidence layer explicit.

## Applicability and rollout

Hygiene resolution is deterministic:

- mirror or archived repositories are `not_applicable` and do not receive synthetic failures;
- generated-only or dormant repositories are advisory;
- active standard and template repositories are required; and
- a reviewed exception affects only the explicit `exception` disposition.

In `observe`, error and critical diagnostics are capped at warning while semantic status remains
`invalid` or `incomplete`. In `ratchet`, diagnostics already present at the supplied base are capped
while new regressions remain blocking. In `enforce`, effective error and critical diagnostics
block. This preserves honest status without pretending a nonblocking rollout is conformant.

## Evidence and reports

The dedicated JSON artifact is
`.reports/egolint/repository-continuity.json` and conforms to
`egolint.repository-continuity-report/v1`. It contains:

- immutable Hygiene and Aether selections;
- applicability and rollout stage;
- requested/resolved base and head states, topology, shallow status, change results, and parallel
  counts;
- distinct structural, freshness-declaration, local-Git, and external-live evidence layers;
- semantic status independent of effective severity;
- ordered privacy-safe diagnostics with stable rule IDs, expected/actual state, remediation,
  contract ownership, and normalized locations; and
- bounded counts for Relay, Pace, and Observatory.

The normal run report and SARIF receive the equivalent normalized findings. The CLI's existing
ordered problem-matcher lines are the deterministic human-readable view; no checkpoint prose or
secret-shaped value is echoed there.

Stable rule families are cataloged in
`.config/rules/repository-continuity.v1.toml`. Required-file failures reuse
`EGO-CONTRACT-FILE-001`; focused rules cover contract pins, schema, structure, identity, links,
agent blocks, freshness, comparison, parallel work, exceptions, external live state, and safety.

## Authoring and handoff boundary

EgoLint is read-only with respect to `CONTINUITY.md`. An authorized person or agent applies the
repository's continuity workflow after domain validation and before presenting or updating a pull
request, then includes the reconciled checkpoint in that same bounded change. CI validates the
result; it does not manufacture semantic prose. A post-merge reconciliation is another explicit
repository change, not a merge status inferred by this offline validator.
