-------------------------------------------------------------------------------
-- Ego Hygiene — Universal Luacheck Configuration
-------------------------------------------------------------------------------
--
-- Purpose:
--   Provide a strict, portable baseline for first-party Lua while allowing
--   framework-specific globals only in the paths where they are expected.
--
-- References:
--   https://luacheck.readthedocs.io/en/stable/config.html
--   https://luacheck.readthedocs.io/en/stable/warnings.html
-------------------------------------------------------------------------------

-------------------------------------------------------------------------------
-- Core language baseline
-------------------------------------------------------------------------------

-- Support the union of standard globals across Lua 5.1–5.4 and LuaJIT.
--
-- This is appropriate for a shared repository containing scripts and examples
-- that may target different Lua runtimes. Individual projects can narrow this
-- through a more specific local configuration later.
std = "max"

-------------------------------------------------------------------------------
-- Diagnostic output
-------------------------------------------------------------------------------

-- Include warning and error codes in output so findings are stable and easier
-- to triage in CI reports.
codes = true

-- Disable terminal coloring because MegaLinter owns report presentation.
color = false

-- Suppress output for files with no findings.
quiet = 1

-------------------------------------------------------------------------------
-- Analysis policy
-------------------------------------------------------------------------------

-- Keep undefined-global detection meaningful.
allow_defined = false
allow_defined_top = false

-- Analyze implicit self arguments.
self = true

-- Detect unused variables, arguments, values, and secondary return values.
unused = true
unused_args = true
unused_secondaries = true

-- Detect shadowing and redefinition.
redefined = true

-- Allow inline Luacheck directives such as:
--   -- luacheck: ignore 212
--   -- luacheck: push ignore 211
--   -- luacheck: pop
--
-- `no_inline = false` is the default but is declared explicitly as policy.
no_inline = false

-------------------------------------------------------------------------------
-- Formatting policy
-------------------------------------------------------------------------------

max_line_length = 120
max_code_line_length = 120
max_string_line_length = 160
max_comment_line_length = 120

-------------------------------------------------------------------------------
-- Performance
-------------------------------------------------------------------------------

-- MegaLinter invokes Luacheck once per file. A repository cache mounted through
-- repeated container runs is not guaranteed, so avoid creating disposable
-- `.luacheckcache` artifacts inside the lint workspace.
cache = false

-------------------------------------------------------------------------------
-- Warning filtering
-------------------------------------------------------------------------------

-- Keep the baseline intentionally narrow. Do not suppress undefined globals,
-- unused arguments, unused loop variables, trailing whitespace, or long lines
-- globally.
--
-- Framework-specific or intentionally unused values should be handled with:
--   - underscore-prefixed variables;
--   - inline directives;
--   - scoped path overrides below.
ignore = {}

-------------------------------------------------------------------------------
-- Busted test files
-------------------------------------------------------------------------------

-- Luacheck already provides default Busted detection for conventional
-- `spec` and `test` paths. These overrides add a few common repository forms.
files["**/spec/**/*.lua"].std = "+busted"
files["**/spec/**/*_spec.lua"].std = "+busted"
files["**/test/**/*.lua"].std = "+busted"
files["**/tests/**/*.lua"].std = "+busted"
files["**/tests/**/*_spec.lua"].std = "+busted"

-------------------------------------------------------------------------------
-- Neovim configuration and plugins
-------------------------------------------------------------------------------

-- `vim` is available only to Neovim-owned Lua sources, not arbitrary Lua files.
files["**/.config/nvim/**/*.lua"].read_globals = { "vim" }
files["**/nvim/**/*.lua"].read_globals = { "vim" }
files["**/neovim/**/*.lua"].read_globals = { "vim" }

-------------------------------------------------------------------------------
-- LÖVE projects
-------------------------------------------------------------------------------

-- Luacheck includes a built-in `love` standard. Add it only inside paths that
-- are clearly owned by a LÖVE project.
files["**/love/**/*.lua"].std = "+love"
files["**/love2d/**/*.lua"].std = "+love"
files["**/löve/**/*.lua"].std = "+love"

-------------------------------------------------------------------------------
-- LuaRocks specifications
-------------------------------------------------------------------------------

files["**/*.rockspec"].std = "+rockspec"
