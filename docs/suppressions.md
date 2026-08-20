# Suppressions

Egolint uses the shared versioned `Suppression` contract. Beyond the structural
JSON Schema, the portability suppression engine requires:

- a stable ID and exact tool/rule identity;
- a workspace-relative path and/or exact finding fingerprint;
- a named owner;
- a nonempty justification;
- an `expires-on` date in `YYYY-MM-DD` form;
- reviewed evidence.

The schema represents evidence as a typed array; the engine rejects an empty
array before matching a portability suppression.

Rule-only blanket suppressions are rejected. Multiple suppressions matching one
finding are rejected rather than resolved by order. An active match records the
suppression ID on the finding and changes the declaration state to `applied`.
Declarations that do not match remain `unmatched`. A declaration remains active
through its expiry date and becomes `expired` the following day; an expired
declaration never hides a finding and emits the blocking
`EGO-SUPPRESSION-EXPIRED` finding. Unmatched declarations emit the nonblocking
`EGO-SUPPRESSION-UNMATCHED` warning, so obsolete policy cannot disappear inside
an otherwise clean report.

The portability catalog determines which rules are suppressible. Consumers
cannot use the generic application API to weaken a rule that its owning rule
pack marks non-suppressible.
