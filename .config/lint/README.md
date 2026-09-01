# Linter configuration catalog

Each directory owns the native configuration for one language, file format, or quality dimension.
`.mega-linter.yml` maps MegaLinter descriptor keys to these files explicitly, while `tasks/lint.yml`
exposes focused commands for local use.

Keep rules tool-native whenever possible. MegaLinter should orchestrate a linter, not redefine its
policy through opaque command-line overrides.

Configuration changes should include at least one matching fixture or a documented reason why an
executable fixture is impractical.

Tools outside MegaLinter are cataloged in
[`../toolchain/complementary-tools.json`](../toolchain/complementary-tools.json). That manifest
records native configuration paths, exact versions, applicability markers, positive and negative
fixture evidence, direct commands, and report destinations. It does not replace native tool
configuration.

Project-aware tools require every marker group in their applicability contract. For example, a
`pyproject.toml` without packaged `src/**/*.py` source does not activate deptry, Vulture,
interrogate, or complexipy. Fixture paths never count as repository applicability.
