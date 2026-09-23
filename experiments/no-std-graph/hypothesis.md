# E1: `no_std` graph hypothesis

This document is pre-run. It contains hypotheses, controls, limits, and
commands only; it contains no observed results.

## Hypotheses

- H1: The graph crate's own resolver compiles as `no_std` after changing only
  the crate attributes and collection/string/vector imports from `std` to
  `alloc`. The resolver algorithm remains unchanged.
- H2: The copied original property and oracle tests exercise the same resolver
  behaviour under the proposal.
- H3: A direct reference to `std::collections::BTreeMap` behind the
  `negative-std-probe` feature makes the feature build fail.

H1 is deliberately limited to the experiment crate's own resolver. The
experiment directly depends on the production `document` and `library`
crates, so the full production dependency closure remains `std`-using through
those crates and their dependencies. This is intentional coupling to the
real production types and dependency closure for fidelity, rather than a
local model or toy implementation.

This does not claim embedded suitability, heap freedom, or panic freedom. It
also does not claim that the complete production graph is end-to-end
`no_std`.

## Input-domain qualification

The production `resolve` function assumes that `PageId` values supplied by
`Corpus` are unique. Duplicate inputs are therefore outside H2's permutation
domain. The copied oracle deduplicates generated ids before parsing pages, and
the copied property generator creates unique page names.

## Run order

1. Commit only this hypothesis and the package manifest before execution.
2. Copy the production source into `src/lib.rs`, add only `#![no_std]` while
   retaining the original `std` imports, and run the naive control. Its
   expected outcome is nonzero.
3. Replace the naive source with the `alloc` proposal in this directory.
4. Run the positive library check, copied tests, and negative feature probe.

The measured commands are:

```sh
cargo check --manifest-path experiments/no-std-graph/Cargo.toml --lib --locked
cargo test --manifest-path experiments/no-std-graph/Cargo.toml --locked
cargo check --manifest-path experiments/no-std-graph/Cargo.toml --lib --locked --features negative-std-probe
```

The third command is expected to be nonzero. No command result is asserted
here before execution.

## Source and transformation checks

Record the source hashes after the proposal is copied:

```sh
sha256sum crates/graph/src/lib.rs experiments/no-std-graph/src/lib.rs
cmp crates/graph/tests/oracle_resolve.rs experiments/no-std-graph/tests/oracle_resolve.rs
cmp crates/graph/tests/properties.rs experiments/no-std-graph/tests/properties.rs
```

The source transformation is compared deterministically with:

```sh
diff -u \
  <(awk '
    /^#!\[forbid\(/ { print "#![no_std]" }
    /^\)\]$/ { print; print "extern crate alloc;"; next }
    /^use std::collections::\{BTreeMap, BTreeSet\};$/ {
      print "use alloc::collections::{BTreeMap, BTreeSet};"
      print "use alloc::{string::String, vec::Vec};"
      next
    }
    /^use library::PageId;$/ {
      print
      print ""
      print "#[cfg(feature = \"negative-std-probe\")]"
      print "fn negative_std_probe() {"
      print "    let _: std::collections::BTreeMap<(), ()> = std::collections::BTreeMap::new();"
      print "}"
      next
    }
    { print }
  ' crates/graph/src/lib.rs) \
  experiments/no-std-graph/src/lib.rs
```
