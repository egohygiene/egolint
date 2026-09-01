# EgoLint Identity Brand Kit

EgoLint is a governed Identity v1 consumer. Human-reviewed source intent lives under `.identity/`;
deterministic projections live under `assets/identity/`. Generated assets must not be hand-edited.

## Immutable inputs

- Identity release: `v1.0.0`
- Linux archive SHA-256: `f8a961790b959fc683b215729c20c743399c7b1d4d8aadf8b1a30e8c3663532a`
- Additive repository-presentation commit: `3c2fd3141371b355628e81f66f63159f19d63338`
- `validate_identity.py` at that commit:
  `eb0c7f9df31b2bd3eb618d830c5d11d76c750d0ef51b9b79092aa72ccfdcbb4f`
- `render_repository_presentation.py` at that commit:
  `5d73593ab3e04baf60d577119defd4049cf2da571f00a9d033d7a7d9430b7396`
- Hygiene repository-presentation profile commit: `28f9d6c7519d820644572634ba4476614f418d83`
- Hygiene profile SHA-256: `44e0881519350e6747723995939c79c6fb4659e38a74b2c32e409866e7a186ba`

The verification workflow downloads and checks these inputs before use. It does not execute mutable
default-branch scripts.

## Regenerate

After reviewing and approving a canonical change beneath `.identity/`:

```sh
identity v1-generate --repository-root "."

PYTHONPATH="path/to/pinned/identity/scripts" \
  python3 "path/to/pinned/identity/scripts/render_repository_presentation.py" \
    --repository-root "." \
    --evidence "evidence/repository-presentation.json" \
    --output "assets/identity/repository-presentation"

identity v1-verify --repository-root "."
```

Review the entire `.identity/` and `assets/identity/` diff together. Confirm that alternative text,
status language, provenance, approvals, represented commit, checksums, and rendered
light/dark/high-contrast/narrow variants remain truthful.

## Upgrade

1. Review the Identity release notes and contract changes.
2. Update the release, commit, and digest pins together.
3. Regenerate into a separate worktree or temporary output directory.
4. Validate canonical source and compare the complete generated tree.
5. Review visual and machine-readable changes before replacing the committed package.
6. Merge the source, pins, evidence, and generated output atomically.

## Roll back

Restore the previous `.identity/`, `vendor/hygiene/`, `evidence/repository-presentation.json`,
workflow pins, and `assets/identity/` tree from Git as one set. Then rerun canonical validation,
`v1-verify`, and repository-presentation reproducibility. Do not repair or partially restore
generated files by hand.

The Identity system governs presentation assets; it does not certify EgoLint, claim that every
capability ran, or change the repository's actual release and support state.
