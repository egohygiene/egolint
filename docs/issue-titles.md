# Canonical issue titles

`egolint issue-title` checks an observed title and label set or formats an explicit
type and reviewed subject. It runs offline and prints deterministic JSON to stdout.
It uses the same pure Rust API exposed by `egolint::issue_titles::IssueTitlePolicy`.

The selected organization definition is **a candidate**, from
[.github PR #45](https://github.com/egohygiene/.github/pull/45) at
`19d2be9bf0191710508cefbb9f0b1abb3a40d9be`. A conformant title means it matches
that definition; it does not establish acceptance, adoption, or enforcement.
This implementation is tracked by [#78](https://github.com/egohygiene/egolint/issues/78).

## Check an observed snapshot

Prepare a normalized JSON file. The provider adapter must observe the title and
**full** label set before asserting `complete: true`:

```json
{
  "schema_version": 1,
  "complete": true,
  "title": "✨ [feature] [FLO-OBS-01] Export reports",
  "labels": ["type:feature", "priority:p2"]
}
```

```sh
egolint issue-title validate --input issue.json
```

`--input` is relative to `--workspace` unless absolute. This command does not
load repository lint configuration, enumerate repository files, start a container,
or contact GitHub. Redirect stdout explicitly if a report needs to be saved.

| Result | Meaning | Exit |
| --- | --- | --- |
| `conformant` | One primary type and its exact title format | 0 |
| `nonconformant` | One primary type, incorrect title format | 1 |
| `needs-classification` | No known primary type label | 1 |
| `conflict` | Multiple distinct known primary type labels | 1 |
| `unsupported-type` | Any unknown `type:*` label | 1 |
| `unavailable` | Snapshot explicitly marked incomplete | 2 |

Unknown types take precedence over missing or conflicting known types. Labels
are treated as a set; their order and duplicates cannot change the result.
Expressive emoji labels and GitHub's separate native issue-type field cannot
substitute for the canonical `type:*` label.

Every input field is required. Missing/null/malformed fields, unsupported versions,
and unknown fields fail with exit 2 and no successful report. An observed empty
label array is different from a missing array. Missing input files and unavailable
contract evidence also fail with exit 2; other I/O errors use Egolint's existing
internal-error category (4). Diagnostics go to stderr.

## Format a reviewed subject

```sh
egolint issue-title format \
  --type maintenance \
  --reviewed-subject '[Release checkpoint 7] Provide pinned tools'
```

The proposal includes `🧹 [maintenance] [Release checkpoint 7] Provide pinned tools`
and the required label `type:maintenance`. It preserves the subject byte-for-byte,
including case, tracking IDs, accents, emoji, and internal spacing. Empty subjects,
outer whitespace, and line breaks are rejected rather than silently repaired.
Exact emoji matching includes variation selectors.

This command does not parse an existing title or strip its prefixes. For a legacy
or duplicated prefix, review the intended subject explicitly before calling it.
Repeating the same type and reviewed subject produces identical output. A future
provider adapter must compare that proposal with the live title and skip no-op
writes; that adapter is outside this checkpoint.

## Provenance and schemas

[The source lock](../.config/rules/issue-title-sources.v1.json) selects the definition,
JSON Schema, label catalog, semantics, and examples from one immutable upstream
commit. The [vendored source tree](../vendor/github/issue-titles/) preserves their
relative paths. Every artifact's SHA-256 is checked before either operation.
The loader also rejects incompatible versions, rule identifiers, adoption semantics,
mapping/catalog disagreement, missing sources, and mixed or moving revisions.
There is no network fallback or runtime override for this initial consumer.

Both results carry the revision, candidate authority, contract version, and all
five digests. They omit runtime timestamps and host paths. Upgrading the pin is a
reviewed source change: fetch all five artifacts at one revision, refresh digests,
verify upstream structure/semantics, rerun fixtures, and review the diff. Editing
the lock alone is not proof that a new revision is accepted.

`egolint schema issue-title-snapshot`, `issue-title-report`, and
`issue-title-proposal` emit the checked-in Rust-derived schemas. `task schemas:check`
and CI detect projection drift. JSON Schema describes structure; the Rust consumer
enforces the behavioral rules. The upstream JSON Schema is bundled unchanged.

## Checkpoint and next integration

The Rust suite executes all 18 upstream validation cases through the library and
CLI, both reviewed migration examples, malformed/incomplete input, classification
precedence, Unicode, deterministic output, source-integrity failures, and schema
projection checks. Run `cargo test --all-targets --all-features --frozen --offline`
after populating the locked dependency cache.

This adds an explicit local check, not automatic repository enrollment. The next
checkpoint is Aether issue-authoring and local agent discovery. Relay then owns
stable issue identity, observation evidence, reviewed preview/apply, conflict checks,
receipts, rollback, and event integration. Promotion requires upstream contract
review and an explicit pin/status update, followed by one repository trial before
the fleet sweep. Labels, templates, `AGENTS.md` projections, ongoing workflows,
aggregate findings/SARIF, and provider writes remain separate integration work.
