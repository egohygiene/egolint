# JSONSkooma capability evaluation

## Decision

**Defer a JSONSkooma-specific EgoLint adapter.** Keep JSON Schema contracts language-neutral and
continue using the existing V8R boundary for filename/catalog-oriented validation. Evaluate a
language-neutral offline CLI or native Rust adapter before adding another language-specific schema
engine. Reconsider JSONSkooma when a real Ruby consumer needs Ruby-local custom dialect,
vocabulary, keyword, or application-parity behavior that the canonical validator cannot provide.

This decision adds no package, image layer, runtime, network access, or execution path. It is not a
rejection of JSONSkooma as a Ruby library. It rejects speculative product integration without a
distinct capability owner or consumer.

## Evaluation snapshot

| Field                  | Evidence                                                                                                        |
| ---------------------- | --------------------------------------------------------------------------------------------------------------- |
| Evaluation date        | 2026-09-19                                                                                                      |
| Owning issue           | [EgoLint #14](https://github.com/egohygiene/egolint/issues/14)                                                  |
| Upstream source        | [`skryukov/json_skooma`](https://github.com/skryukov/json_skooma) at `973af58de8ec47ca736f510e5662e4d4dd29d463` |
| Evaluated release      | `json_skooma` `0.2.7`, published 2026-06-10                                                                     |
| Release integrity      | RubyGems SHA-256 `518494acf0dcbfc8d6de0924730524365222ffe4b70211effdc588c79a9ce8df`                             |
| License                | MIT                                                                                                             |
| Required Ruby          | `>= 2.6`; upstream CI covers MRI 3.0 through 4.0, JRuby, and TruffleRuby                                        |
| Packaged gem           | 38,400-byte archive; five direct runtime dependencies                                                           |
| Duplicate EgoLint work | No JSONSkooma source, configuration, or pull request found at evaluation time                                   |

The upstream `0.2.6` package was yanked because its JSON Schema metaschema submodules were omitted;
`0.2.7` repaired the package. That incident is resolved, but it demonstrates why any future adapter
must verify installed bytes and exercise a packaged-gem smoke test rather than relying only on the
source checkout.

## Fleet applicability

The accessible organization default branches contained no `Gemfile`, `.gemspec`, or
`.ruby-version` at evaluation time. Four Ruby files were indexed: a Homebrew formula, workflow
helpers, and a linter fixture. None declares a Ruby application or a Ruby-local JSON Schema
contract. By contrast, JSON Schema usage is broad: the same bounded search found hundreds of
`$schema` references and more than one hundred `*.schema.json` files across multiple repositories.

This inventory supports a language-neutral schema capability now. It does not justify treating the
current absence of a Ruby application as a permanent veto. Plausible future consumers include:

- a Ruby service, gem, Rails application, or Ruby MCP server that validates contracts in process;
- a Ruby application with custom dialects, vocabularies, keywords, formats, or schema resolvers;
- a repository that must prove parity between application-time JSONSkooma behavior and CI; or
- a mixed-language repository that explicitly selects a Ruby schema capability for a supported
  dialect.

GitHub code search covers indexed default-branch content visible to the linked account. It is a
bounded inventory, not proof that private, unindexed, generated, historical, or future content does
not exist.

## Capability assessment

### JSONSkooma strengths

- Implements the 2019-09 and 2020-12 dialects.
- Exposes custom dialect, vocabulary, keyword, format, source, and output-formatter extension
  points.
- Emits JSON Schema-style `instanceLocation`, `keywordLocation`, and
  `absoluteKeywordLocation` fields in basic, detailed, and verbose output.
- Uses the official JSON Schema Test Suite for required assertions and most optional/format cases.
- Can resolve references through caller-supplied local or remote sources.
- Is actively maintained, MIT licensed, compact as a gem, and supports multiple Ruby runtimes.

### Product gaps

- JSONSkooma is a library, not a first-party CLI. EgoLint would own argument parsing, file loading,
  YAML/JSON handling, process exit semantics, timeouts, output stability, redaction, and JSON/SARIF
  normalization.
- The library does not expose a stable standalone rule identifier. An adapter could derive a
  keyword from `keywordLocation`, but that derived contract would belong to EgoLint.
- Upstream does not publish a representative performance benchmark. Safe runtime budgets would
  require an EgoLint-owned corpus and cold/warm measurements.
- Optional suite exclusions include ECMAScript regular-expression behavior, unknown-keyword
  references, dependency-compatibility cases, and one 2019-09 cross-draft case. Those exclusions
  matter for cross-validator parity.
- Remote sources are possible. EgoLint would have to reject them by default and accept only pinned,
  local resolution unless a separately authorized network capability exists.
- The lightweight CLI image contains no Ruby. The full image currently inherits Ruby for RuboCop,
  but an incidental base-image runtime is not a stable JSONSkooma packaging contract.

## Cross-language comparison

| Candidate                            | Primary fit                               | Drafts/features                                                                    | CLI and evidence                                          | EgoLint disposition                                                                   |
| ------------------------------------ | ----------------------------------------- | ---------------------------------------------------------------------------------- | --------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| V8R 6.1.0 / Ajv 8                    | Filename, SchemaStore, JSON/YAML/TOML     | Ajv-backed; custom schemas/catalogs                                                | Existing MegaLinter adapter and JSON output               | Keep for catalog-oriented validation; remove implicit network reliance over time.     |
| `check-jsonschema` 0.38.0            | Language-neutral repository CLI           | Python `jsonschema` 4.x, metaschema mode, custom validator class, vendored schemas | First-party CLI, JSON output, pre-commit integration      | Stronger near-term candidate for explicit schema-to-instance mappings.                |
| Rust `jsonschema` / `jsonschema-cli` | Native or standalone offline validation   | Drafts 4, 6, 7, 2019-09, 2020-12; custom keywords/formats; structured output       | CLI plus native library; Rust 1.85-compatible line exists | Preferred experiment if native ownership and binary-size measurements are acceptable. |
| Ajv directly                         | JavaScript application/runtime validation | Drafts 4, 6, 7, 2019-09, 2020-12; strong extension model                           | Library; separate `ajv-cli` product                       | Do not duplicate V8R without a concrete application-parity requirement.               |
| Python `jsonschema`                  | Python application/runtime validation     | Multi-draft library and extension protocols                                        | Library; already exposed through `check-jsonschema`       | Use through the CLI boundary unless Python-local parity is required.                  |
| Dart `json_schema` 5.2.2             | Dart/Flutter application validation       | Through Draft 7                                                                    | Library, platform-oriented diagnostics                    | Consumer-side option; insufficient as the canonical modern-dialect validator.         |
| Ruby JSONSchemer 2.5.0               | Broad Ruby/OpenAPI validation             | Drafts 4, 6, 7, 2019-09, 2020-12; OpenAPI 3.0/3.1                                  | Library with structured output                            | More mature Ruby comparator; still does not remove the adapter cost.                  |
| JSONSkooma 0.2.7                     | Extensible Ruby-local validation          | 2019-09 and 2020-12; custom dialects/vocabularies                                  | Library with detailed structured results, no CLI          | Defer until its Ruby-specific extension model is a demonstrated requirement.          |

The languages of these implementations do not redefine the canonical schemas. JSON Schema files
and explicit schema-to-instance mappings remain the portable source of truth.

## Cost and bounded execution

The selected defer decision has a measured product cost of **zero added bytes and zero added
runtime**: EgoLint does not install the gem, initialize Ruby, or execute JSONSkooma.

A future prototype starts with the following known lower bound:

- one 38,400-byte `json_skooma` 0.2.7 archive;
- five direct runtime dependencies (`bigdecimal`, `hana`, `regexp_parser`, `uri-idna`, and
  `zeitwerk`) plus resolved transitive dependencies;
- a pinned Ruby runtime where it is not already part of the selected image;
- an EgoLint-owned executable adapter and normalized evidence tests; and
- packaged-gem, schema-suite, cold-start, warm-validation, memory, and timeout measurements.

No throughput claim is recorded because upstream publishes no representative benchmark and the
evaluation environment did not contain Ruby. Inventing a number would be weaker evidence than the
explicit re-evaluation gate below. If prototyped, compare at least small configuration schemas,
large generated schemas, deep/nested failures, reference-heavy inputs, and hostile depth/size
limits against the selected language-neutral validator.

## Re-evaluation and applicability contract

Reopen the decision only when at least one trigger exists:

1. An owned Ruby project declares JSON Schema validation in its runtime or build contract.
2. A supported schema requires a custom dialect, vocabulary, keyword, or result annotation that
   the canonical language-neutral validator cannot preserve.
3. Application/CI parity requires the same Ruby engine in both paths.
4. A measured prototype shows material precision, diagnostic, or runtime value that exceeds its
   packaging and maintenance cost.

If reconsidered, activation must require all of these signals:

- a Ruby manifest or an explicit, reviewed Ruby capability declaration;
- one or more JSON Schema documents or explicit schema-to-instance mappings;
- a supported 2019-09 or 2020-12 dialect;
- a selected schema/Ruby capability or an explicit repository override; and
- a pinned adapter, gem dependency graph, and local-only schema source set.

Non-Ruby repositories, Ruby-only utility files, schema-absent repositories, unsupported dialects,
and repositories that disable the capability must report `skipped` or `not-applicable`. They must
not initialize Ruby or install gems at lint time.

A future adapter must prove valid, invalid, non-Ruby, schema-absent, explicit-enable,
explicit-disable, unsupported-dialect, local-reference, hostile-depth, timeout, and redaction
fixtures. Normalized evidence must preserve instance path, schema/keyword path, derived keyword or
rule, dialect, adapter version, gem version, configuration digest, and represented commit.

## Sources

- [JSONSkooma README](https://github.com/skryukov/json_skooma/blob/973af58de8ec47ca736f510e5662e4d4dd29d463/README.md)
- [JSONSkooma gem specification](https://github.com/skryukov/json_skooma/blob/973af58de8ec47ca736f510e5662e4d4dd29d463/json_skooma.gemspec)
- [JSONSkooma test-suite adapter](https://github.com/skryukov/json_skooma/blob/973af58de8ec47ca736f510e5662e4d4dd29d463/spec/json_skooma_spec.rb)
- [JSONSkooma 0.2.7 release](https://github.com/skryukov/json_skooma/releases/tag/v0.2.7)
- [V8R](https://github.com/chris48s/v8r)
- [`check-jsonschema`](https://github.com/python-jsonschema/check-jsonschema)
- [Rust `jsonschema`](https://github.com/Stranger6667/jsonschema)
- [Ajv](https://github.com/ajv-validator/ajv)
- [Dart `json_schema`](https://github.com/Workiva/json_schema)
- [Ruby JSONSchemer](https://github.com/davishmcclurg/json_schemer)
