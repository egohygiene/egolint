# ADR-002 ratification record

This document maps Hygiene issue
[#15](https://github.com/egohygiene/hygiene/issues/15) to reviewable contract
evidence and records the human authority that accepted ADR-002 and policy
v1.1.0.

## Acceptance evidence

| Issue requirement | Canonical evidence |
| --- | --- |
| Versioned global specification | [`POLICY.md`](POLICY.md) and `egohygiene.architecture-decision/v1` |
| Explicit required and optional fields | Policy section 5 and [`architecture-decision.v1.schema.json`](../../schemas/architecture-decision.v1.schema.json) |
| Lifecycle and supersession semantics | Policy sections 9-10 plus accepted, rejected, and invalid approval fixtures |
| Organization and repository ownership | Policy section 2 and ADR-002 |
| Inheritance by reference and version | Policy section 6 and `egohygiene.architecture-decision-policy-reference/v1` |
| Permitted extensions and invalid overrides | Policy section 7, both schemas, and hostile fixtures |
| Stable IDs and index requirements | Policy section 4 |
| Relay/Egolint validation requirements | [`VALIDATION.md`](VALIDATION.md) and [`tools/decisions.py`](../../tools/decisions.py) |
| Existing-repository migration | [`MIGRATION.md`](MIGRATION.md) |

The matrix established that the proposal was complete enough to decide. The
ratification evidence below supplies the separate human disposition.

## Ratification evidence

- **Decision:** `egohygiene/hygiene#ADR-002`
- **Policy:** Organization Architecture Decision Record Policy v1.1.0
- **Reviewed implementation:** `f598ed659a43dd759d4ede41c27f9e5daf991aa7`
- **Approved by:** [`szmyty`](https://github.com/szmyty)
- **Date:** 2026-09-12
- **Durable evidence:** [Hygiene #15 approval comment](https://github.com/egohygiene/hygiene/issues/15#issuecomment-5647398908)
- **Validation evidence:** [Hygiene #15 ratification packet](https://github.com/egohygiene/hygiene/issues/15#issuecomment-5647360172)
- **Activation review:** [Hygiene PR #48](https://github.com/egohygiene/hygiene/pull/48)

The approval states:

```text
I approve egohygiene/hygiene ADR-002 and the Organization Architecture
Decision Record Policy v1.1.0 at commit
f598ed659a43dd759d4ede41c27f9e5daf991aa7 for organization use.
```

## Activation recorded by this change

- [x] ADR-002 is `accepted` with the named authority and exact evidence URL.
- [x] The policy document is `accepted`.
- [x] The ADR and policy-reference contracts are active in the organization
      contract index.
- [x] The decision indexes and roadmap gate reflect ratification.
- [x] Contract, fixture, repository, and generated-file checks were rerun.

This record does not convert implementation, merge, CI success, or agent output
into human authority. Future disposition changes require their own explicit,
durable approval evidence.
