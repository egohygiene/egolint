# Repository Intelligence semantic validation

Egolint validates the repository-owned records that feed roadmap, decision, and delivery-history
projections. The validator is offline and deterministic: Hygiene owns the source contracts, Egolint
owns semantic rules and normalized diagnostics, and Relay may later orchestrate the released CLI
without reimplementing those rules.

Catalog `0.1.0-alpha.2` consumes the ratified Hygiene ADR policy. Its two ADR contract pins have
`accepted` authority; the roadmap and projection contracts retain their separate `proposed` pins.
Policy acceptance never supplies a consumer decision's human approval or changes its lifecycle.

## Ratified ADR compatibility

The ADR source revision is `c589587395750cd1c79c6fa0bef010189c547249` in `egohygiene/hygiene`.
Its [ratification record](https://github.com/egohygiene/hygiene/blob/c589587395750cd1c79c6fa0bef010189c547249/docs/decisions/RATIFICATION.md)
records explicit human approval and activation through Hygiene PR #48. The policy is accepted and
the contract catalog marks both ADR contracts active; this is the basis for `accepted` authority,
rather than an inference from merged implementation.

| Contract | Version | Authority | Source revision |
| --- | --- | --- | --- |
| `egohygiene.architecture-decision/v1` | `1.1.0` | `accepted` | `c589587395750cd1c79c6fa0bef010189c547249` |
| `egohygiene.architecture-decision-policy-reference/v1` | `1.0.0` | `accepted` | `c589587395750cd1c79c6fa0bef010189c547249` |
| `hygiene.roadmap/v1alpha1` | `0.1.0` | `proposed` | `f598ed659a43dd759d4ede41c27f9e5daf991aa7` |
| `egohygiene.repository-intelligence/v1` | `1.0.0-alpha.1` | `proposed` | `f598ed659a43dd759d4ede41c27f9e5daf991aa7` |

The two ADR schema files are byte-identical between these revisions. Their SHA-256 digests are:

| Source path | SHA-256 |
| --- | --- |
| `schemas/architecture-decision.v1.schema.json` | `491eae26e498ee311bb37d5eff23a51d682eb687196c80a3df10ac1ce98e04d4` |
| `schemas/architecture-decision-policy-reference.v1.schema.json` | `fdf451cfa38ba666ec579aa75d2acbd52a56b95e73c7437565b79dfcfb689216` |

Consumer migration requires updating both ADR entries in the local TOML policy to the exact
accepted pins, plus `policy.source.revision` in the local policy-reference JSON. Contract versions,
source paths, decision schemas, diagnostic IDs, and report schema version 1 remain unchanged.
The bundled fixture shows the complete supported tuple for every contract. The native validator
does not fetch, rewrite, or silently upgrade consumer policy.

Older proposed ADR pins, arbitrary revisions, moving refs, and mismatched versions, authority,
repositories, or paths produce `EGO-INTEL-CONTRACT-001`. Historical replay can use a separately
pinned older EgoLint build; that is not a ratified-policy conformance claim. Advisory enforcement
still records invalid or incomplete semantics even when findings do not block execution.

Relay adoption is a separate review gate: pin the reviewed EgoLint commit, refresh the runtime
lock and verified source digests, and rerun its native ADR fixtures before removing the explicit
policy-compatibility limitation. This change does not itself refresh Relay, release EgoLint, enable
required mode, validate diagram semantics, or authorize fleet rollout. Legacy and unknown ADR
coverage remain incomplete.

## Inputs and outputs

A repository supplies one TOML policy and one represented source revision:

```sh
egolint validate \
  --repository-intelligence ".config/egolint/repository-intelligence.toml" \
  --represented-commit "$(git rev-parse --verify HEAD)"
```

`--represented-commit` accepts only a full lowercase commit SHA, `unknown`, or `not-applicable`. An
exact SHA is resolved from the local checkout; validation never fetches a branch, schema, issue,
pull request, or commit from the network. Shallow or explicitly bounded commit history remains
`incomplete` rather than being reported as complete evidence.

The command continues to write the canonical run report and SARIF. When this policy is selected it
also writes:

```text
.reports/egolint/repository-intelligence.json
```

That version-1 artifact contains the selected profile and contract pins, represented commit, source
coverage, semantic validity, stable rule IDs, effective severities, source locations, remediation,
and exact record counts. Relay and Observatory can decode these fields directly; they do not need to
parse console text. Git diagnostics use the synthetic source path `@git/<full-sha>` plus the
one-based commit-message line.

The composite action exposes matching `repository-intelligence` and `represented-commit` inputs plus
a `repository-intelligence-report` output. If the policy is selected and the input is empty, the
action uses `GITHUB_SHA`.

## Versioned policy

The complete positive policy fixture is
[`tests/fixtures/repository-intelligence/valid/policy.toml`](../tests/fixtures/repository-intelligence/valid/policy.toml).
Its top-level surfaces are:

- `profile`: a stable local name, `blocking` or `advisory` enforcement, and the exact enabled
  Egolint rule IDs;
- `contracts`: immutable versions, authority states, source repositories, 40-character revisions,
  and paths matching the bundled catalog;
- `adrs`: `present`, `unknown`, or `not-applicable`, canonical local paths, and explicitly pinned
  external decision identities;
- `roadmap`: the same adoption state, canonical `ROADMAP.md`, and declared external dependency
  identities; and
- `commit-history`: the same adoption state plus a deterministic maximum commit count.

Only rules named by `profile.enabled-rules` execute. `blocking` preserves each catalog severity and
fails on error/critical findings. `advisory` converts enabled policy failures to warnings while the
dedicated artifact still records semantic status as `invalid` or `incomplete`. Disabled rules
produce no hidden pass claim.

`unknown` is a visible incomplete migration state. `not-applicable` is an explicit repository
assertion. `present` requires the source to exist and validate. This distinction lets a repository
adopt one surface at a time without treating missing ADR history as successful ADR conformance.

## Rule catalog

The embedded [`repository-intelligence.v1.toml`](../.config/rules/repository-intelligence.v1.toml)
maps every rule to its Egolint owner, Hygiene contract IDs, default severity, and structured
remediation.

| Rule                              | Meaning                                                                                              |
| --------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `EGO-INTEL-CONTRACT-001`          | Exact supported Hygiene versions, authority states, paths, and immutable revisions.                  |
| `EGO-INTEL-ADOPTION-001`          | Explicit source availability, represented commit, and bounded/shallow history.                       |
| `EGO-INTEL-ADR-METADATA-001`      | Safe first front matter, required v1 fields, filename/ID agreement, extensions, and section anatomy. |
| `EGO-INTEL-ADR-LIFECYCLE-001`     | Decision/implementation states, human disposition evidence, verified evidence, and exceptions.       |
| `EGO-INTEL-ADR-INDEX-001`         | Exactly one canonical relative index link per ADR, including terminal history.                       |
| `EGO-INTEL-ADR-LINEAGE-001`       | Resolvable relations and bidirectional accepted supersession.                                        |
| `EGO-INTEL-ROADMAP-STRUCTURE-001` | One v1alpha1 manifest plus unique stable steps, outcomes, and checklist criteria.                    |
| `EGO-INTEL-ROADMAP-STATE-001`     | Metadata/body state agreement, completion claims, and dependency readiness.                          |
| `EGO-INTEL-LINK-001`              | Structural issue, pull request, commit, dependency, and decision references.                         |
| `EGO-INTEL-TRAILER-001`           | Optional `Roadmap-Step:` and `ADR-Ref:` trailers when present.                                       |
| `EGO-INTEL-CYCLE-001`             | Acyclic roadmap dependency and ADR supersession graphs.                                              |

GitHub URLs are validated structurally only. Reachability, visibility, and live state belong to a
separate read-only evidence collector so an offline semantic run cannot leak protected content or
vary with network state.

## Fixtures and trust boundary

Positive fixtures include a proposed ADR, a valid accepted/superseded pair, a two-step roadmap,
local and declared external references, and valid commit trailers. Hostile fixtures cover invalid
lifecycle authority, duplicate and dangling ADR identities, index drift, malformed URLs,
inconsistent roadmap states, missing dependencies, malformed trailers, and both graph cycle types.

Ratification regressions additionally cover every contract-pin field, old and moving policy
references, independent draft-contract authority, deterministic mismatch reports, and an implemented
accepted decision without explicit approval. An implemented proposal may remain proposed. The
native checks can be repeated with `cargo test --locked --lib repository_intelligence`.

Repository-owned Markdown remains canonical. The JSON report is generated evidence and must not be
edited into a competing decision or roadmap source.
