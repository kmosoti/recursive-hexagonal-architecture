# Module-test source-boundary fixture

This is an independent, authored registration package for the additive
module-test source boundary. It contains twelve small control cases. The
cases are a bounded hand-layout domain, not a claim to implement or test a
complete Rust parser: sources contain only conventional `mod name;`, the
registered `#[path] mod` form, and the registered root-call body. The positive
nested case uses a real `src/checks.rs` conventional module followed by an
unannotated `#[path] mod shared;` declaration.

`CASES.json` is the contract-shaped input. Every physical file used by a case
is present in its `files` map; symlink targets are relative entries in its
`symlinks` map. Paths are virtual case-relative paths and never host-absolute.
The refused external files intentionally contain invalid Rust. The reference
checks confinement before inspecting those contents, so the marker tests
boundary-before-parse ordering rather than parser quality.

`reference.py` has two modes:

* With no arguments it is read-only. It validates the exact row shape, input
  closure, canonical simulated paths, inherited-test allowance, positive and
  negative hand controls, expected outcomes, physical source digests, and the
  comment-only digest mutation. It also checks the seven payload hashes.
* `python3 reference.py --reproduce CHILD` first performs the same validation,
  then writes all nine package files to a fresh direct child of this package.
  The child must not already exist; its parent is not created. The command
  compares every resulting byte with the frozen package inputs and removes the
  child if reproduction fails.

For a staging copy, copy this directory as one unit into a fresh staging
directory, preserve the nine files and relative paths, and run the default
reference command there. To exercise byte identity, run reproduction from the
staged package into a new direct child, compare or inspect its result, then
remove that deliberately created child. Do not add `__pycache__`, scratch
files, generated source, or host-dependent paths.

The omitted-test-root control is explicit in the reference: a workspace path
outside the crate remains disallowed when the test root is `None`. A production
context likewise never receives the workspace allowance. The reference first
validates that the crate lies beneath the configured workspace root. It grants
the wider boundary only for exact declaration-level `#[cfg(test)]`; an inner
`#![cfg(test)]`, `cfg(any(test, feature = "x"))`, or `cfg_attr` does not grant
access. Logical module names and the helper-to-`x` test edge are asserted
independently of the expected JSON values, while the digest comparison asserts
that a comment-only helper change changes the bound physical-file digest
without changing that edge.

The three snapshots preserve the source material used to author this package:
`contractaddendum.md`, `originalcontract.md`, and `prompt.md`. Registration
metadata measures those frozen snapshots and the exact `SHA256SUMS` bytes; it
does not require equality with future live source documents.
