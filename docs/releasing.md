# Release design

No release pipeline or public artifact is claimed to be active in this alpha.
`dist-workspace.toml` records the intended binary distribution shape; generated
automation should be reviewed before it is committed.

## Planned products

| Product | Planned channel |
| --- | --- |
| `egolint` crate and binary | crates.io |
| Platform archives and installers | GitHub Releases via cargo-dist |
| Lightweight CLI image | `ghcr.io/egohygiene/egolint` |
| MegaLinter engine/policy image | `ghcr.io/egohygiene/egolint-full` |

## Promotion checklist

1. Update the version and changelog from one reviewed commit.
2. Run formatting, tests, policy validation, and both local image smoke tests.
   This is currently a blocking, unverified gate because the authoring
   workspace did not provide Docker or Podman.
3. Confirm crate contents with `cargo package --list` and a dry run.
4. Run cargo-dist planning and inspect its generated workflow.
5. Use a crates.io trusted publisher rather than a long-lived API token.
6. Build OCI manifests for `linux/amd64` and `linux/arm64` from the same commit.
7. Pin every container base, including MegaLinter and Node, by reviewed
   multi-architecture manifest digest.
8. Generate CycloneDX/SPDX SBOMs and build provenance for binaries and images.
9. Sign image digests with keyless Sigstore/Cosign after provenance succeeds.
10. Publish immutable version tags first; move convenience tags only after
    verification.
11. Verify downloaded archives and `oci://` images against the repository's
    attestations before announcing the release.

The cargo-dist configuration enables GitHub attestations, auditable Rust
binaries, and CycloneDX metadata. OCI publishing remains a separate workflow so
the AGPL-3.0-only full-image boundary and upstream digest review are explicit.

GitHub Actions used by generated or handwritten workflows must be pinned to
reviewed immutable commit SHAs. The initial `.github/workflows/ci.yml` checks the
Rust MSRV, package contents, generated schemas, policy contracts, Node lockfile,
commit policy, lightweight image, and a native-architecture full-image build
with embedded Python and Node policy smokes. Full multi-architecture image
verification and release publication remain explicit later gates.
