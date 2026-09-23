# Test-module source-boundary decision

## Decision

Permit a Rust module source outside its package root only when all of these are
true:

1. its module declaration is already known, before reading the child file, to
   be under an exact `#[cfg(test)]` declaration ancestry;
2. the canonical child path is under an explicitly supplied canonical test
   source root; and
3. the package root is itself under that test source root.

For full-workspace `cargo xtask architecture`, the permitted test source root is
the subject's `CrateGraph.workspace_root`, not the xtask checkout root. Production
modules remain confined to the canonical package root. The root module remains
confined to the package root. Module-only mode and the existing public extractor
remain package-root-strict.

This implements the existing requirement to follow legal `#[path]` modules and
retain test edges without allowing production source to escape its package. The
physical file does not acquire ownership from its directory: the shared helper is
still the logical module
`site::assembly::transclusion_tests::registered_package`, and every edge under
that ancestry remains `test_only`. Its canonical bytes must be included in
`source_files_sha256`.

An inner `#![cfg(test)]` in an external file cannot grant access because reading
it would cross the boundary first. A non-exact condition such as `cfg(any(test,
...))` or `cfg_attr` remains a limitation and cannot grant the wider root.
Canonicalization precedes the containment decision, so a symlink inside the
workspace whose target is outside it is refused before source contents are read.

## Minimal API and wiring

Keep the existing strict API unchanged:

```rust
pub fn extract(
    crate_root: &Path,
    root_file: &Path,
    crate_name: &str,
    edition: &str,
    externals: &BTreeSet<String>,
) -> Result<Extracted, String>;
```

Add one scoped entry point which delegates to the same private implementation:

```rust
pub fn extract_with_test_root(
    crate_root: &Path,
    root_file: &Path,
    permitted_test_root: &Path,
    crate_name: &str,
    edition: &str,
    externals: &BTreeSet<String>,
) -> Result<Extracted, String>;
```

The private implementation canonicalizes both roots once, rejects a package root
outside the permitted root, and stores `Option<PathBuf>` on `Engine`. In
`discover_file`, after canonicalizing the candidate and before `read_to_string`,
the predicate is exactly:

```text
candidate under crate_root
OR
(inherited_test AND candidate under permitted_test_root)
```

Do not use the child's file-level attributes to make that decision. Keep module
keys derived from `ChildSpec.path`; never derive them from the external physical
path.

Likewise keep `workspace::source_files(crate_root, extracted)` strict and add
`source_files_with_test_root(crate_root, permitted_test_root, extracted)`. It
canonicalizes and validates both roots and every deduplicated physical module
source before reading it, permits only package-root or permitted-root paths, and
hashes every accepted file under its canonical absolute path. Extraction is the
authority that established that every accepted non-package path was test-only;
the hash layer must not discover or widen paths.

`workspace::apply` calls the two scoped functions with
`&graph.workspace_root`. `single::report` continues calling the strict functions.
No change is needed in `architecture.rs`, metadata, rules, report schema, site
sources, Cargo manifests, or `rha-modules.toml`.

Boundary errors remain configuration errors. Use distinct messages for a
production module outside the package root and a test-only module outside the
permitted root so negative controls cannot pass for the wrong reason.

## Pre-code registration and controls

This is a read-authority clarification and must be recorded before verifier code.
Add the paragraph above to `docs/architecture/module-check-contract.md`, record
the delegated decision and the exact controls in the active task/change record,
and freeze a new additive boundary-control registration. Do not edit or reinterpret
the existing M, L-M, EM, or X cases or their expected grading.

Register these cases:

1. **Nested inherited test source succeeds.** A package-root file declares
   `#[cfg(test)] mod tests;`; that test file declares an unannotated `#[path]`
   child under the workspace root but outside the package. Scoped extraction
   finds the child's logical module, records its internal reference as
   `test_only`, and emits no production finding.
2. **Production workspace escape fails before parse.** The same workspace-local
   child path without test ancestry points to syntactically invalid Rust. Scoped
   extraction returns the production-boundary error, not a parse error.
3. **Test workspace escape fails before parse.** A test-only `#[path]` points
   beyond the workspace to syntactically invalid Rust. The test-root boundary
   error wins. On Unix, repeat through a workspace-local symlink to the outside
   file and require the same result.
4. **Default remains strict.** Calling existing `extract` on case 1 returns the
   crate-root boundary error. This binds module-only mode and direct extractor
   callers to their old boundary.
5. **Provenance is complete.** Scoped source hashing contains the canonical
   external helper exactly once; changing only that helper changes its digest.
   All logical modules remain mapped to the declaring crate paths.
6. **Live integration closes the blocker.** The existing live-site architecture
   test exits zero, the site module check is `passed`, the canonical
   `xtask/tests/support/registered_package.rs` key is present in the site's
   `source_files_sha256`, and test-originated edges remain notes/test-only.

The first five belong in focused module-extraction/workspace tests; the sixth
strengthens `xtask/tests/modules_cli.rs`. Preserve the existing production escape
and symlink controls as independent strict-boundary regressions.

## Alternatives assessed

| Alternative | Correctness | Boundary safety | Provenance | Compatibility | Change cost | Total |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Scoped workspace root for test-only modules | 5 | 5 | 5 | 5 | 4 | **24/25** |
| Dedicated workspace test-support crate | 5 | 5 | 4 | 2 | 2 | **18/25** |
| Move/copy the helper under the site package | 3 | 5 | 2 | 3 | 3 | **16/25** |

A dedicated crate would give the helper Cargo ownership, but it creates a public
test-support API, workspace role, dev-dependency and policy changes for a source
module Rust already permits. Moving the shared helper under `site` falsely makes
one consumer own cross-owner corpus verification; copying it creates divergent
digest pins and safety logic. `include!` would convert an observed legal module
into a declared extraction hole, and skipping test modules would remove M11
evidence, so neither is an acceptable repair.

## Bounded touched set

- Pre-code: `docs/architecture/module-check-contract.md`, the active task/change
  decision, and a new additive boundary-control registration.
- Tool: `xtask/src/modules/extract.rs`, `xtask/src/modules/workspace.rs`.
- Assurance: focused boundary tests in `xtask/tests/module_extraction.rs` (or one
  new narrow test file) and the live assertion in `xtask/tests/modules_cli.rs`.

The frozen corpora, `xtask/src/modules/single.rs`, all production crates, shared
helper bytes, report schemas, and existing grading expectations remain unchanged.
