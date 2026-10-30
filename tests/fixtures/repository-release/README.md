# Current-v1 repository release fixtures

These synthetic, offline consumers exercise the pinned Aether declaration and Hygiene policy.
`profiles.json` inventories nine distinct source profiles; each directory owns its declaration and
version-authority bytes. `common` supplies shared local entrypoint and changelog source. Archived
consumers omit Taskfile and workflow files because the pinned lifecycle marks those slots not applicable.

The Rust fixture uses Cargo, Python uses PEP 621 metadata, npm and site use package metadata, container
uses an explicit image-version selector, publication uses YAML metadata, workspace keeps independent
workspace/client authorities, contract-only uses a catalog record, and archived uses a frozen Git-tag
authority. A static Git-tag authority does not prove that a tag exists.

The workflow's repeated `a` commit is a synthetic immutable-reference grammar control. It does not
identify an authentic checkout action revision and must never be executed. No fixture builds, publishes,
dispatches, contacts a registry, or initializes a language/container publication runtime. Native
validation inspects source bytes only; `network_access: not_performed` and
`external_publication: not_verified` remain explicit in expected evidence.

The matrix test will retain six states per profile: compliant, advisory migration, unavailable evidence,
external ownership, invalid/nonconforming, and caller-selected not applicable. Shared state mutations
will preserve the profile-specific authority and lifecycle distinctions rather than relabeling one
generic repository.
