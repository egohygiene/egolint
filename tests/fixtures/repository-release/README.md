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

The matrix test retains six states per profile: compliant, advisory migration, unavailable evidence,
external ownership, invalid/nonconforming, and caller-selected not applicable. Shared mutations preserve
the profile-specific authority and lifecycle distinctions rather than relabeling one generic repository.

| State | Source variation | Expected boundary |
| --- | --- | --- |
| Compliant | Complete profile-specific source | Every applicable check completed and passed. |
| Advisory migration | Caller selects advisory; root changelog is missing | The missing check remains visible as a warning and incomplete coverage. |
| Unavailable | Selected authority is missing; archived loses its changelog | Required evidence remains unavailable and blocks. |
| External | Source/delivery evidence explicitly belongs externally; active profiles use an immutable reusable-workflow reference | Local wrapper structure passes, while external ownership remains explicit and reachability is unverified. |
| Invalid | Authority has a leading-zero semantic version; archived has a nonconforming promoted heading | Completed failed checks make semantic status invalid. |
| Not applicable | Caller selects not applicable; local entrypoints and authority files are removed | All seven slots remain explicitly not applicable. |

Archived retains the archived lifecycle in every cell. Its Task/workflow checks are not applicable and
its agent pointer remains advisory under the selected policy. Explicit caller adoption overrides use the
existing contract; tests do not alter the owner profile to manufacture a state. An externally owned
Task include is unavailable to this static validator, while external workflow delivery can be represented
through a pinned local wrapper and supported evidence fields. `automation.github.state: external` is
invalid under the exact selected declaration grammar.

`expected-reports.json` contains all 54 reviewed compact focused projections: profile, lifecycle,
adoption, every slot's status/requirement, coverage counters and explicit evidence boundary. It omits
duplicated prose and byte digests. The same cases separately assert process exit, tool status/counts,
normalized finding severity/location and SARIF identity. Complete coverage means the applicable checks
ran; it does not turn a completed failing check into conformance.

Additional cases cover matching/drifted released authorities in all seven single-component local
profiles; independent workspace versions; missing/malformed/unpinned/external Task/workflow source;
malformed declarations and tag constants; and issue 91's echo/wrong-workflow boundaries. The inherited
issue 91 suite retains its full static dispatch grammar matrix.

On Unix, trusted failing executables trap Docker, Podman, Node, pnpm, Python, GitHub CLI, Task and Cargo
initialization while the actual native CLI validates contract-only and archived consumers. A harmless
trap invocation proves the sentinel works before each native check. This verifies orchestration only;
it is not real container, language-runtime or release-adapter qualification.
