# Adaptive capability selection

Issue [#35](https://github.com/egohygiene/egolint/issues/35) adds a versioned
selection layer to the existing planner. `egolint capabilities` prints the
compiled registry; `egolint plan --format json` explains each capability for a
particular checkout. Plans and run reports retain the decisions separately from
actual tool results. Selection never means execution or a passing check.

The existing pinned MegaLinter catalog, profile snapshots and tool matrix remain
the owners of tool versions and profile defaults. The registry composes them
with the native capabilities, complementary-tool inventory and optional
ast-grep adapter. It does not copy another organization's policy or execute
command arrays from the complementary catalog.

## Decisions and boundaries

Every decision contains a stable ID/version/provider, placement, `available`,
`applicable`, state and fixed reason. The state vocabulary distinguishes
available, applicable, selected, skipped, unavailable and invalid. Invalid
configuration or required metadata returns the existing configuration error;
it does not become a successful skip.
Explicitly disabled and out-of-profile capabilities do not inspect project
manifests; their `applicable` flag is false with the policy skip reason. A
metadata predicate stops once a matching project is established, so unrelated
fixture manifests cannot invalidate an already matched project.

For full-image tools, availability means declared in the compiled reviewed
catalog, not a probe of a running image. Host-adapter availability is executable
discovery; the adapter must still verify its version when invoked. Native tools
are compiled into the CLI. Existing complementary integrations are explicitly
configuration-only in this execution boundary; their independent owners and
commands remain unchanged. A declared capability without an executor is shown
as unavailable, never passed.

The predicate schema supports portable file globs, bounded JSON/TOML metadata,
runtime presence, lifecycle events, declarations, services, release-comparison
context, and all/any composition. Known project-aware tools require matching
project metadata. Release-runtime capabilities must require a release lifecycle
and verified comparison context; none is enabled by this change. The existing
native release-declaration validator remains a static source check and still
runs outside release events.

Git-visible tracked and non-ignored regular files are inventoried; a bounded
filesystem walk supports non-Git planning. Symlinks and conventional dependency/
report/cache directories do not become source inputs. The inventory rejects
more than 100,000 candidates and excessive fallback depth; manifest reads have
a 2 MiB limit. Full profiles retain whole-repository scope. `fast` retains its
changed-file engine setting plus whole-repository native policy.

## Configuration and migration

```toml
config-version = 1

[capabilities]
enabled = true
disable = ["COPYPASTE_JSCPD"]
declarations = ["AST_GREP"]
services = []
```

Adaptation defaults on. Adding supported source activates applicable tools
within the existing profile, without changing CI. Repository layers merge
exclusions/declarations/services; omitted fields retain inherited values.
Overrides do not silently replace the compiled profile. CLI enable/disable
overlays remain explicit, and applicability/safety requirements still apply.

Network-oriented schema/link checks require both an explicit tool declaration
and the named `external-http` service, with container network access explicitly
enabled. Installing an active scanner or runtime is insufficient. Service
entries are names, not endpoints, and planning does not probe a service.

`[capabilities] enabled = false` explicitly restores legacy profile selection.
The original profile-overlay assertions remain exercised under that switch.
Custom MegaLinter policy remains subject to adaptive exclusions; use the
explicit migration switch when a reviewed legacy policy requires its previous
selection semantics. Fix preview retains its bounded explicit formatter path
and is independent of automatic check selection.

The new optional report fields and regenerated closed schemas are additive
Rust/Serde inputs, but consumers validating against an older closed schema must
update their selected schema before accepting new output. No release or image
publication is implied. Physical image/tool qualification and first-release
acceptance remain separate from source implementation under #5.

## Adding a capability

Before extending the registry, record these inclusion decisions with its owner:

- License compatibility, upstream maintenance and an exact reviewed version.
- Parser/runtime dependencies, package provenance and independently verified
  platform digests; installation alone is not execution authority.
- Incremental image size and startup cost. Common bounded static tools belong
  in the full image; heavy or uncommon stacks need a specialized image or
  explicit host adapter. Configuration-only integrations remain inventory.
- Network and service requirements, explicit opt-in, bounded time/output and
  deterministic unavailable/skip behavior. Active probes require declarations.
- Overlap with existing tools and the owner of each rule, normalization and
  suppression behavior. Avoid duplicate findings without an explicit reason.
- Positive, hostile, empty, generated-only and evolving/polyglot controls,
  plus reproducible performance measurements before widening defaults.

The [ast-grep evaluation](ast-grep-evaluation.md) applies these criteria to the
new optional host adapter. Its advisory placement does not expand the full image.
