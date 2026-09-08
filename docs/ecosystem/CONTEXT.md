<!-- egohygiene-context: repository-context/v2 -->
---
schema-version: "2.0.0"
context-version: "2.0.0"
architecture-release: "architecture-v0.1.0"
repository: "egohygiene/egolint"
source-repository: "egohygiene/hygiene"
source-revision: "1c720954283b91134c18a7cfa28e5c2dda505d46"
generated-by: "egohygiene/hygiene:repository-context@2.0.0"
continuity-policy: "egohygiene.repository-continuity-policy/v1@1.0.0-alpha.1"
---

# Ecosystem context for `egohygiene/egolint`

> Generated from the pinned Hygiene catalog. Do not edit this projection by hand.

## Identity

- Plane: `developer-runtime`
- Visibility: `public`
- Lifecycle: `incubating`
- Maturity: `early-implementation`

## Ownership

### Owns

- lint CLI
- lint profiles
- rules
- MegaLinter integration
- report normalization
- safe autofix policy

### Does not own

- workflow fleet distribution
- unrelated project templates

### Publishes

- quality reports
- SARIF
- bounded autofix patches

## Dependencies

### Repository inputs

- None declared.

### Consumed contracts and artifacts

- None declared.

## Neighbors

### Upstream

- None declared.

### Downstream

- egohygiene/empathy
- egohygiene/observatory
- egohygiene/relay

## Constraints

- Integrate sibling capabilities through released, immutable artifacts instead of copied source.
- Preserve repository-owned content and generated-file provenance.
- Do not place credentials, private identity data, or secret material in repository projections.
- Treat this context as a generated projection; change ecosystem ownership in Hygiene instead.
- Do not absorb or claim ownership of workflow fleet distribution.
- Do not absorb or claim ownership of unrelated project templates.

## Repository continuity

- Policy: `egohygiene.repository-continuity-policy/v1@1.0.0-alpha.1` (`proposed`).
- Portable contract: `aether.repository-continuity/v1`.
- Repository-owned checkpoint: `CONTINUITY.md`.
- Repository-owned instructions: `AGENTS.md` with exactly one managed Aether pointer block.
- Resume: read and reconcile the checkpoint before selecting work.
- Handoff: refresh it after validation and before pull-request presentation.
- Static instructions do not install an automatic pre-pull-request hook.

## Canonical links

- [Agent Context](https://github.com/egohygiene/hygiene/blob/1c720954283b91134c18a7cfa28e5c2dda505d46/docs/ecosystem/AGENT_CONTEXT.md)
- [Architecture](https://github.com/egohygiene/hygiene/blob/1c720954283b91134c18a7cfa28e5c2dda505d46/docs/ecosystem/ARCHITECTURE.md)
- [Catalog](https://github.com/egohygiene/hygiene/blob/1c720954283b91134c18a7cfa28e5c2dda505d46/catalog/repositories.yaml)
- [Decisions](https://github.com/egohygiene/hygiene/blob/1c720954283b91134c18a7cfa28e5c2dda505d46/docs/decisions/README.md)
- [Migration](https://github.com/egohygiene/hygiene/blob/1c720954283b91134c18a7cfa28e5c2dda505d46/docs/ecosystem/MIGRATION_PLAN.md)
- [Repository](https://github.com/egohygiene/egolint)

## Upgrade and stale-context behavior

- Compare `architecture_release` and `continuity_policy` with the selected Hygiene release.
- On mismatch: `fail`.
- Upgrade owner: `egohygiene/pace`.
- Action: Regenerate from the pinned Hygiene release and review the resulting pull request.
