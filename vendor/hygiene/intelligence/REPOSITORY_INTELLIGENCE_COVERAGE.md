# Repository Intelligence collection coverage

Status: **proposed alpha.2**, under [ADR-005](../decisions/ADR-005-unify-repository-intelligence-projection.md).
Implements the input boundary needed by [Observatory #25](https://github.com/egohygiene/observatory/issues/25).

An empty array does not say whether its source was checked. Projection
`1.0.0-alpha.2` requires `collection_coverage` with exactly nine domains.
The [alpha.2 schema](../../schemas/repository-intelligence.alpha2.schema.json)
and reference validator own these semantics. Hygiene retains ownership;
Egolint owns production semantic lint, Relay collects, and Observatory preserves
these claims in repository, view, and fleet output. No ownership changes.

## Scope and evidence

Each entry describes collection of the projection's **root repository** at the
represented revision and observation time. External graph nodes provide context;
they are not an inventory of another repository. A collector may include only
authorized, publishable records. Denied repositories must be excluded before
projection assembly; never emit a named placeholder or withheld-repository count.

| Domain | Collection scope |
| --- | --- |
| `roadmap` | Canonical roadmap steps at the represented revision. |
| `decisions` | All ADR records at the represented revision, including proposed and historical records. |
| `git` | Commit inventory reachable from the represented revision; shallow or limited history is partial. |
| `issues` | Complete authorized issue inventory, including open and closed; referenced issues alone are partial. |
| `pull_requests` | Complete authorized pull-request inventory, including open, closed, and merged. |
| `checks` | Check inventory for the represented revision. |
| `releases` | Repository release inventory through the observation time. |
| `deployments` | Repository deployment inventory through the observation time. |
| `history` | Collected lifecycle events across the above domains through the observation time; a bounded window or subset of domains is partial. |

Inventory coverage does not establish history coverage. In particular, querying
current issue states does not prove that issue lifecycle events were collected.
Graph events refer to existing entities as before. No placeholder issue or
evidence entity may be fabricated to satisfy a dangling reference. Authored,
public issue URLs may remain in a roadmap step's `attributes.issue_references`
array without asserting the issue's existence, visibility, or provider state.

## Required entry

Every domain has exactly `collection`, `freshness`, `reason`, and `observed_at`.
No free text, identity, URL, provider payload, count, or extension is permitted in
a coverage entry. Diagnostics identify the fixed domain, never the rejected value.

| Collection | Meaning | Allowed reason | Freshness | Observation |
| --- | --- | --- | --- | --- |
| `uncollected` | This domain was not requested. | `not_requested` | `unknown` | null |
| `unavailable` | No usable authorized observation. | `access_denied`, `provider_unavailable` | `unknown` | null |
| `partial` | Some scope was attempted; completeness is not established, even if zero records returned. | `filtered`, `truncated`, `incomplete` | `current`, `stale` | required |
| `observed_empty` | Complete authorized collection found no records in this domain. | `complete` | `current`, `stale` | required |
| `observed` | Complete authorized collection found records. | `complete` | `current`, `stale` | required |
| `failed` | Collection failed without a usable observation. | `collection_failed` | `unknown` | null |
| `not_applicable` | An authorized repository declaration explicitly excludes this domain. | `explicit_not_applicable` | `not_applicable` | required |

Observation times are valid UTC RFC 3339 timestamps ending in `Z`, no later than
the projection's `observed_at`. They are collector-supplied; normalization never
refreshes them. Not-applicable uses the declaration observation time; absence of
a declaration cannot establish it. Collectors retain the declaration evidence
in their collection evidence artifact; coverage does not publish arbitrary URLs.

`observed` requires represented root-domain entities (`history` uses events on
root entities). `observed_empty`, `uncollected`, `unavailable`, `failed`, and
`not_applicable` forbid those records. Sources may still record a successful
empty collection. Retained usable evidence from an incomplete or failed refresh
is `partial` with its actual observation and freshness, never a fresh empty set.
Coverage freshness is independent of individual record freshness and conformance;
`observed` is not a claim that checks passed or policy requirements were met.
A stale empty inventory cannot establish a current zero.

Pagination must finish, all authorized states must be included, and no filtering,
redaction of domain records, or access-limited scope may remain before a collector
claims complete. A successful HTTP response or empty filtered result is insufficient.
This contract does not by itself award achievements or prove a live provider was checked.

## Exact compatibility and migration

- `schemas/repository-intelligence.v1.schema.json`, the graph vocabulary at
  `1.0.0-alpha.1`, and `fixtures/repository-intelligence/complete-quest.json` stay
  unchanged. They remain the exact legacy compatibility artifacts.
- Alpha.2 uses `schemas/repository-intelligence.alpha2.schema.json` and the same
  alpha.1 graph vocabulary. Only collection coverage changes; graph identity,
  canonical intent, authority, and event meaning are unchanged.
- The reference validator accepts exactly alpha.1 and alpha.2. Alpha.1 cannot
  carry the new field. Alpha.2 requires every domain; arbitrary versions fail.
- Old consumers must reject alpha.2 on their exact version check. Coverage is
  deliberately required and versioned because ignoring it would be misleading.
- A new consumer may accept alpha.1 only with explicitly **unknown collection
  completeness**, preserving the existing graph. It must not infer a full
  collection from the fixture name, positive records, or empty arrays.
- Repin schema, reference fixtures, validator and vocabulary to one immutable
  Hygiene revision; repin Observatory and renderer contracts together after
  compatibility review. Existing EgoLint/Relay alpha.1 pins remain fail-closed
  until their owners add alpha.2 support. Merging this proposal does not activate
  downstream publication or ratify the proposed Intelligence contract.

The eight deterministic examples in
[`fixtures/repository-intelligence/coverage`](../../fixtures/repository-intelligence/coverage)
cover full, roadmap-only, denied, partial, failed, observed-empty, stale, and
explicit not-applicable collection. They are synthetic and carry no protected
repository identities. Observatory supplies the corresponding view/fleet goldens
and Relay repinning guide; provider collection is a separate task.
