# Canonical issue titles

Contract: `egohygiene.issue-title/v1`, version `1.0.0`.
Owner: `egohygiene/.github`.
Tracking: [contract checkpoint #44](https://github.com/egohygiene/.github/issues/44),
[parent #24](https://github.com/egohygiene/.github/issues/24), and
[fleet reconciliation #23](https://github.com/egohygiene/.github/issues/23).

The [machine-readable contract](../.github/issues/title-contract.v1.json)
owns the type-to-emoji mapping and rule identifiers. This document owns their
meaning. The [label catalog](../.github/labels/catalog.v1.json) continues to
own exact label names, descriptions, colors, and category membership.
Changes to these sources must be reviewed together when their meanings change.

## Format and primary type

An issue title is `{emoji} [{type}] {subject}`. Use exactly one ASCII space at
each prefix boundary, the contract's exact emoji codepoints, and the lowercase
type token. The subject is nonempty, has no leading/trailing whitespace, and
contains no line breaks. Prefer a short, descriptive action for new work.
Do not automatically rewrite case or prose during migration.

Choose exactly one primary universal `type:*` label. Other labels remain
independent. Expressive overlay labels such as `🐛 bug` do not substitute for
`type:bug`. GitHub's native issue-type field is also a separate provider
field; it does not override this contract's primary label.

The six mappings are in the contract. Illustrative titles include:

- `🐛 [bug] Fix duplicate archive ingestion`
- `✨ [feature] Add repository achievement tracking`
- `📄 [documentation] Explain local setup`
- `🧹 [maintenance] [Release checkpoint 7] Provide pinned tools`

Keep tracking IDs and meaningful checkpoint/roadmap text within the subject.
Parentage, dependencies, and completion are determined by stable issue identity
and relationships, never by parsing this prefix. Issue titles do not change
commit messages, pull-request titles, or release semantics.

## Classification and validation order

For a complete observed issue snapshot, evaluate in this order:

1. If any label beginning `type:` is absent from the pinned mapping, report
   `unsupported-type`.
2. If no known primary type is present, report `needs-classification`.
3. If multiple distinct known primary types are present, report `conflict`.
4. With exactly one known type, compare the title against its required prefix
   and the subject rules. Report `conformant` or `nonconformant`.

Treat labels as a set. Never infer the primary type from title decoration,
choose the first label, or silently fall back to maintenance. Missing source
access, unavailable labels, and incomplete snapshots are capability/evidence
limitations, not an empty label set or a passing result.

A maintainer or agent can propose a classification from issue content.
Record that decision in the reviewed migration plan. Formatting is then
deterministic; semantic classification is not part of the formatter.

## Existing-issue normalization

A formatter takes an explicit primary type and a reviewed subject and produces
the canonical title. Before applying to an existing issue, capture its stable
identity, original title and labels, observed revision/update evidence,
contract revision/digest, selected type, reviewed subject, and expected output.

A normalizer may recognize one complete, already-canonical prefix and retain
the subject byte-for-byte. For legacy, duplicated, malformed, or unknown
prefixes, require an explicit reviewed subject. Do not strip arbitrary emoji,
bracketed tracking IDs, or checkpoint markers. Changing wording beyond the
prefix must be an explicit item in the plan.

Retain unrelated labels and all issue bodies, discussion, assignments,
milestones, state, and relationships. Recheck the current title and labels
before each write; changed inputs become conflicts rather than overwrites.
Record successful updates for resume. Rollback restores the recorded title and
any managed type-label changes only when current state still matches the
applied result.

A repeat against conformant state produces zero mutations. The
[synthetic cases](../fixtures/issue-titles/cases.v1.json) describe expected
validation statuses and reviewed migration examples for downstream consumers.

## Agent entry point and adoption

[AGENTS.md](../AGENTS.md) is the local discovery entry point. Before authorized
issue creation or editing, an agent reads this document and the mapping,
selects a primary type, verifies provider label availability, and prepares a
conforming title. Unavailable provider access must be reported rather than
invented. Instructions do not grant write, merge, or publication authority.

This source change establishes a contract candidate for review. Once merged,
it establishes the approved definition; it does not prove provider enforcement
or silently enroll other repositories.

Consumers pin the contract, semantics, catalog, schema, and examples from one
immutable source revision and record that revision in their adoption evidence.
The catalog version must match the contract reference. An unavailable or
incompatible contract is reported as unavailable; use no silent fallback to
mutable main or an older cached policy.

Initial adoption is observe mode. A repository moves to enforcement only after
its primary labels, templates, local agent discovery, validator, and execution
path are present and evidenced. GitHub does not distribute this repository's
AGENTS.md to every agent: consumer repositories need their own explicit pointer
or managed Aether projection. Local template overrides need explicit updates.

## Ownership and staged delivery

| Owner | Responsibility |
| --- | --- |
| .github | Organization title convention, label taxonomy, examples, adoption coordination |
| Hygiene | Organization applicability and contract-index reference |
| Aether | Portable issue-authoring guidance and managed agent projections |
| Egolint | Reusable issue-title validation and deterministic formatting |
| Relay | GitHub preview/apply, event handling, receipts, and recovery |
| Pace / Observatory | Adoption and drift evidence through existing boundaries |

Checkpoint #44 supplies the contract, examples, local entry point, and
structural checks. Consumer implementation and contract-index registration are
follow-on work. The organization work form still has its existing default
until template consumption is implemented; no automatic issue renaming or
ongoing event workflow is installed by this change.

After contract review, prove the formatter/validator and agent consumption,
then a single-repository Relay preview/apply and no-op repeat. Add ongoing
event/template conformance and expand the backlog sweep through #23.
Keep historical issues in a separate recorded batch.

## Contract changes

Editorial clarification preserving accepted outputs is a patch.
Additive optional examples or metadata may be minor. A changed mapping,
classification meaning, required format, or acceptance behavior requires a
major version and migration plan. A consumer upgrade must record the selected
revision; editing this source does not retroactively mutate issues.
