# Module test-source boundary draft review

Scope: the completed pre-code package in `target/m2/module-test-boundary`,
reviewed against `docs/architecture/module-test-source-boundary.md` and
`target/m2/transclusion-helper-boundary-decision.md`. No extractor
implementation or implementation test was read or run.

## Findings repaired before freeze

Two corpus defects were found and corrected during review.

1. Test-only escapes beyond the configured workspace originally expected the
   same `outside crate root` substring as production escapes. That could not
   distinguish a wrong production classification. The direct and canonical
   symlink test-root escapes now expect `outside permitted test source root`;
   strict/default and production-context escapes retain `outside crate root`,
   and invalid crate/test-root configuration retains `crate root`.
2. The inner-attribute, non-exact `cfg(any(...))`, and `cfg_attr` no-grant
   controls originally targeted a file outside the configured workspace. An
   extractor that wrongly classified them as test-only would still refuse at
   the second boundary. Their final targets are
   `workspace/shared/helper.rs`: inside the test root but outside the crate.
   The target is intentionally invalid Rust, so the correct production-boundary
   refusal wins before reading/parsing, while an incorrect test grant cannot
   produce the registered result.

## Final package result

No remaining semantic discrepancy was found in the corrected 12-case package.

- Direct `#[path = "../../shared/helper.rs"]` declarations from
  `workspace/member/src/lib.rs` resolve to `workspace/shared/helper.rs`.
  The inherited case resolves ordinary `mod checks;` to
  `workspace/member/src/checks.rs`, then its unannotated
  `#[path = "../../shared/helper.rs"]` to the same shared helper.
- The inherited case therefore has the correct logical modules `x`,
  `x::checks`, and `x::checks::shared`, with the registered test edge from
  `x::checks::shared` to `x`. Direct accepted cases correctly use
  `x::checks -> x`; physical helper placement never changes logical ownership.
- All symlink targets remain inside their own hypothetical case root. The
  accepted link canonicalizes to `workspace/shared/helper.rs`; the negative link
  canonicalizes to `external/helper.rs` outside the configured workspace; and
  the production symlink remains refused even though its target is within the
  workspace.
- Exact declaration `#[cfg(test)]` is the only wider-root grant. The true
  out-of-line child inherits it without another annotation. An external inner
  attribute, `cfg(any(test, feature = "x"))`, and `cfg_attr` do not grant it.
  The narrower-root case rejects configuration before module traversal because
  the crate root is not contained in the permitted test root.
- Accepted `source_files` maps contain every traversed canonical physical file:
  two files for direct/symlink cases and all three files for the nested case.
  Independently recomputed SHA-256 values match every registered digest. The
  helper-only mutation changes its registered digest while preserving the
  module list and test edge. Refused rows consistently expose no partial module,
  edge, or digest result.

The existing 507 registered transclusion cases and headline module corpus are not referenced or
regraded by this additive package. Metadata and reproduction closure were
audited separately by the root coordinator.
