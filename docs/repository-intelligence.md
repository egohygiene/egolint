# Repository Intelligence semantic validation

Egolint validates the repository-owned records that feed roadmap, decision, and delivery-history
projections. The validator is offline and deterministic: Hygiene owns the source contracts, Egolint
owns semantic rules and normalized diagnostics, and Relay may later orchestrate the released CLI
without reimplementing those rules.

Catalog `0.1.0-alpha.3` consumes the ratified Hygiene ADR policy. Its two ADR contract pins have
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

## Collection coverage compatibility (alpha.2)

Hygiene's [coverage contract](https://github.com/egohygiene/hygiene/blob/639a003d5ddc4d242c2cf190eeb59a9fc522d199/docs/ecosystem/REPOSITORY_INTELLIGENCE_COVERAGE.md)
and Observatory #25 distinguish uncollected, denied, partial, observed-empty,
observed, failed and explicitly not-applicable domains, separately from freshness.
EgoLint now accepts the following **additional exact pin** for source validation:

```toml
[[contracts]]
id = "egohygiene.repository-intelligence/v1"
version = "1.0.0-alpha.2"
authority = "proposed"
source-repository = "egohygiene/hygiene"
source-revision = "639a003d5ddc4d242c2cf190eeb59a9fc522d199"
source-path = "schemas/repository-intelligence.alpha2.schema.json"
```

Replace the existing Intelligence entry when selecting this version; do not add
a duplicate contract ID. Every coordinate must match. The supported alpha.1 pin
above remains valid; ADR and roadmap pins, authority and validation stay unchanged.
The source catalog increments to `0.1.0-alpha.3`; source-policy/report schemas
remain version 1. Merge status does not ratify the proposed Intelligence contract.

After collecting a projection, run the native, read-only coverage check:

```sh
egolint intelligence validate-coverage --input collected-projection.json
egolint schema intelligence-coverage-report
```

The library equivalent is `egolint::intelligence_coverage::validate_coverage`.
It consumes at most 4 MiB of captured JSON, performs no network or provider access,
and emits JSON on stdout without writing files or executing repository code.
The CLI accepts regular files only and rejects symlinks. Relative paths resolve
against `--workspace`; errors never echo the supplied path or JSON payload.
Duplicate JSON keys fail closed. Output is deterministic across capture paths
and record order; no wall-clock timestamp is introduced.

This command's explicit report `scope` is **`collection-coverage`**. It checks
exact input version, root-domain context, all nine coverage domains, required
fields, fixed reason/freshness combinations, UTC observation times and their
upper bound, and contradictions between claims and represented root entities or
events. It reads allowed combinations from the checksum-verified owner schema.
Inventory and lifecycle history remain separate. External records do not count
as observations of the root repository. It does not certify the complete Hygiene
projection schema, graph relationship semantics, source truth, authorization,
provider pagination, freshness policy, or policy conformance.

The owning Hygiene schema/graph check and EgoLint's existing source lint are
therefore still required. A valid coverage report means the supplied claims are
consistent; it cannot prove that a provider was actually queried. In particular,
`partial`, `unavailable`, `uncollected`, `failed`, and stale empty observations
never establish a current zero. Only a collector with a complete authorized
inventory may assert `observed` or `observed_empty`.

| Input/result | Report | Exit |
| --- | --- | --- |
| Supported alpha.2 claims, including partial/uncollected | `status: valid`, `coverage: explicit`, all claims preserved in `domains` | 0 |
| Alpha.1 without coverage | `status: valid`, `coverage: legacy_unknown`, `domains: null` | 0 |
| Malformed, unsupported, missing or contradictory claims | `status: invalid`, fixed diagnostic, no claims | 1 |
| Private/internal/unknown visibility or protected records | `status: unavailable`, fixed diagnostic, no identities/counts/claims | 2 |
| Unreadable, non-regular or oversized file | Fixed bounded input error | 2 |

Exit 0 is not a publication or completeness grant. An alpha.1 graph may contain
many records but cannot acquire inferred collection completeness. Attaching
alpha.2 coverage under alpha.1 is rejected. Failed reports preserve no partial
claims; diagnostics contain fixed codes only, without attacker-controlled keys,
provider messages, URLs, repository names or counts. Even denied inputs return
only the public owner-contract identity and pin, never their repository identity.

The [source lock](../.config/rules/repository-intelligence-coverage-sources.v1.json)
records the exact revision and hashes for the two unchanged owner schemas, graph
vocabulary, coverage specification, legacy fixture and eight alpha.2 fixtures.
Only contracts and public synthetic fixtures are bundled; no sibling implementation
is copied. The runtime verifies its embedded contract bytes before evaluation.
The report schema is generated in CI and the Cargo package includes every input.

### Relay adoption checkpoint

For Relay #115, and the subsequent #112/#113 collection/build integration:

1. Pin the reviewed EgoLint revision and packaged source digests. Select the exact
   alpha.2 alternative above in the source policy, retaining the accepted ADR pins.
2. Validate the immutable ADR/roadmap corpus using the existing
   `validate --repository-intelligence ... --represented-commit ...` command.
3. Assemble captured projection input, run Hygiene's full schema/reference graph
   checks, and invoke `intelligence validate-coverage`. Consume its JSON report
   and exit code directly, without parsing console descriptions.
4. Feed that same validated input to the pinned Observatory alpha.2 normalizer.
   Repin Relay's existing renderer to display domain uncertainty before enabling
   publication. Replay both legacy and alpha.2 cases; never relabel payloads or
   treat `legacy_unknown` as complete coverage.

This checkpoint supplies validator compatibility. Collector integration, renderer
repinning, consumer adoption and deployment remain Relay-owned follow-up work.
