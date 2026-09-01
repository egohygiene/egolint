# Configuration examples

Copy `egolint.toml` to a repository root, then inspect the result before running any linter:

```sh
egolint config explain
egolint plan --format "json"
```

`egolint.local.toml` demonstrates a developer-only override. Local configuration is skipped in CI
and should normally be ignored by version control.

The example image uses the alpha `edge` name because that is the compiled default. No public image
is asserted to exist yet. Once releases begin, pin a reviewed `egolint-full` digest instead of
relying on a moving tag.
