# Transclusion fixture I/O decision

## Decision

Accept the pure/filesystem split as restated by the root. Core grader targets
must consume compile-time embedded bytes and perform no filesystem, environment,
process, or clock operation. Filesystem package verification belongs only to the
already-effectful `app-cli` integration-test target. Do not add any lint allow,
change a core crate's `forbid`, hide calls behind aliases, condition the forbid on
`cfg(test)`, move private analyzers into public APIs, or create a support crate.

The retained Clippy output proves the distinction: graph and site unit modules
inherit their library crate's unconditional `forbid`, while document's integration
target is independently denied by the core `clippy.toml`. The correct repair is to
remove the effects from all three owner graders, not suppress those diagnostics.

## Exact helper split

### `xtask/tests/support/registered_package.rs` — pure, canonical owner

Keep the single `Spec` table, digest pins, `SHA256SUMS` parser, safe relative-path
validation, exact case count/kind checks, and all nested schema validators here.
It may use collections, `library::Digest`, `serde_json`, and byte/string parsing;
it must not import or call `std::fs`, runtime environment/process APIs, or any
other effect.

Expose only test-helper APIs:

```rust
pub fn verify_payloads(
    package: &str,
    payloads: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String>;

pub fn load_from_payloads(
    package: &str,
    payloads: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<Value>, String>;

pub fn load(package: &str) -> Vec<Value>;

pub fn validate_cases(package: &str, value: &Value) -> Result<Vec<Value>, String>;
```

`load_from_payloads` calls `verify_payloads`, parses that map's `CASES.json`, and
then calls `validate_cases`; it must not duplicate binding or shape rules.
`load` obtains the generated embedded map, delegates to `load_from_payloads`, and
panics with package context only because its callers are tests. Unknown packages,
missing/extra files, duplicate paths, wrong bytes, malformed sums, wrong counts,
and schema errors remain fail-closed.

The pure validator checks the exact nine-file inventory per package, both metadata
file pins, every payload digest listed by `SHA256SUMS`, the cases pin, and the
registered case count/kind. There is one binding implementation regardless of
whether bytes came from the compiler or the filesystem.

### `xtask/tests/support/registered_package_embedded.rs` — generated data only

Generate this checked-in module from the four frozen package directories. It
contains the ordered mapping for all 36 files (nine files for each of regions,
sections, sites, and URIs) using `include_bytes!` with paths relative to the
generated source. It contains no validation logic and no manually copied digest
table. Generation measures the existing frozen `registration.toml`,
`SHA256SUMS`, and payload files, refuses symlinks/extra inventory, and emits a
deterministic result; a check mode must detect generated-source drift.

Declare it as an ordinary test-only module from `registered_package.rs` so the
module extractor observes the source. Do not use source `include!` to conceal it.
`include_bytes!` is compile-time data inclusion, not runtime core I/O. Any corpus
byte change either fails the pure digest check or produces a visible regenerated
source diff; the frozen 507 expectations themselves remain unchanged.

### `xtask/tests/support/registered_package_fs.rs` — byte acquisition only

Compile this module only in `crates/app-cli/tests/transclusion.rs`. It owns
`root`, recursive scan, temporary/copied-tree support as needed, and filesystem
reads. It first completes a `symlink_metadata` traversal and rejects symlinks,
unsafe names, and special entries before reading any payload byte. It then builds
one `BTreeMap<String, Vec<u8>>` and delegates to the pure validator.

Prefer a single operation such as:

```rust
pub fn load_at(root: &Path, package: &str) -> Result<Vec<Value>, String>;
```

so the app integration test consumes the same bytes that were scanned and
validated. If a separate `verify_at` is retained for negative controls, it must
share the same scan/read helper; avoid the previous verify-then-reopen sequence.
The module performs no digest, registration, or case-shape decision itself.

## Test ownership and migration

- `crates/graph/src/transclusion_tests.rs` keeps the private region analyzer
  grader and calls only pure `registered_package::load("regions")`.
- `crates/site/src/assembly/transclusion_tests.rs` keeps private assembly and URI
  coverage owner-local. Remove `root`/`verify` calls and use pure embedded loads
  for `sites` and `uris`.
- `crates/document/tests/transclusion.rs` keeps the section/parser projections,
  selector queries, and pure malformed-shape controls. Remove all `std::fs`, temp
  directory, copy-tree, symlink, and runtime package-root code.
- `crates/app-cli/tests/transclusion.rs` includes both the pure helper and the
  filesystem acquisition module. Its existing effectful harness loads registered
  site cases through `load_at`. Move the valid-copy/tamper control and the Unix
  symlink-before-read/precedence control here without weakening their exact error
  assertions.

The filesystem negative controls must still verify the copied valid control
before mutation. The symlink control replaces an expected payload with a link to
byte-identical outside content, poisons another payload for the precedence arm,
and requires the symlink-specific error to win. The pure schema negative retains
its verified control before deleting a required key or adding an unknown key.

## Dependencies and boundaries

No dependency, manifest, lockfile, production API, report schema, or crate is
needed. `library` and `serde_json` are already available to every consuming test
target; app-cli already owns filesystem/process effects. Core library forbids and
all core Clippy templates remain byte-for-byte unchanged.

The workspace test-source boundary extension remains necessary. Site and graph
still declare the pure shared helper, and the generated embedded-data module,
through legal test-only modules physically outside their package roots. They keep
their declaring crate's logical ownership and all extracted edges remain
test-only; removing runtime I/O does not make those module declarations disappear.

## Required controls

1. Exact owner tests remain green over 80 sections, 262 region graphs, 37 parsed
   documents, 12 sites, 153 URI cases, and selector queries, with no fixture-byte
   changes.
2. A pure-loader control runs each of the four embedded packages through
   `load_from_payloads` and proves the expected counts and exact 36-file closure.
3. Mutating one in-memory byte, deleting one path, and adding one path each fail
   the pure validator for the intended reason; malformed case shape reaches and
   fails the schema validator only after a valid binding control.
4. The moved app-cli filesystem controls retain valid-copy, digest-tamper,
   symlink-only, and symlink-before-poison precedence discrimination.
5. Targeted Clippy for document, graph, and site all targets contains no
   disallowed-effect error from either shared helper. No lint allow appears in
   those packages.
6. The module-source-boundary controls and the real site architecture check still
   observe the external pure/helper-data modules, hash their Rust source files,
   retain test-edge notes, and pass. Embedded payloads are data dependencies, not
   falsely reported Rust module sources.

## Bounded touched set

- `xtask/tests/support/registered_package.rs`
- new generated `xtask/tests/support/registered_package_embedded.rs` and its
  deterministic generator/check entry point
- new `xtask/tests/support/registered_package_fs.rs`
- `crates/document/tests/transclusion.rs`
- `crates/graph/src/transclusion_tests.rs`
- `crates/site/src/assembly/transclusion_tests.rs`
- `crates/app-cli/tests/transclusion.rs`
- narrow tool-side generation/integrity test if the generator check is not
  already invoked by an existing test target

Do not touch the four frozen transclusion package bytes, core crate roots or
Clippy configurations, public production modules, Cargo manifests, or the frozen
module/markdown registrations.
