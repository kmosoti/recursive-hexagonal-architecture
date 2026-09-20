# CHG-004: H4 crate-level harness

Task: [CHG-004](../../.rha/tasks/CHG-004-h4-crate-harness.toml). Plan: §6 and §8 W4.
Base: `140fdfb92d62d3481d95f3e2efc1d24a0206fcc4`, merge of PR 6.

## Required order and approval

1. `dedc8ccda78f6b35e5e30ce2cf5db97fdede240b` records CHG-003 acceptance and archives the CI evidence for the accepted merge.
2. This commit changes EM-M03's cell association in the manifest to `cells = ["law3-d1", "law6-b3"]`, with its pinned test row and this task record.
3. Harness implementation follows those commits.

Kennedy approved the protected manifest edit in the working conversation on 2026-09-20: “I approved the EM-M03 cell edit on the manifest: cells = ["law3-d1", "law6-b3"].” Under §9.14 this corrects the test's cell attribution with explicit authority. Every other case and all grading values remain fixed (DP-1.1c).

Kennedy runs his held-out cases at acceptance (DP-1.1b); they are never read by the Executor. Accepted maturity remains his decision (DP-1.6).

## Harness packet

Kennedy's explicit W4 objective requires committed generated fixtures. `cargo xtask corpus generate` materializes the registered crate cases under `xtask/tests/corpus/crate/{violations,legitimate,expected_miss}/<id>/`; outside packages remain beside the associated workspace as their declared paths require. `corpus generate --check` detects added, changed and missing input files without repairing them. Only Cargo output (target directories and lockfiles) is excluded. The committed snapshots are derivative data; the manifest remains their sole semantic owner.

The earlier crate-fixture prohibition in `corpus_manifest.rs` is replaced by the drift gate and complete `corpus_crate.rs` test, explicitly changing the assurance strategy under §9.14 as instructed. Module fixture absence is still checked. No case or grading value changes.

`cargo xtask corpus run --level crate` runs the committed cases using actual `cargo xtask architecture --manifest-path … --rules … --format json` commands and `cargo check --offline` for R01. H4 schema 2 combines all 22 detect cases while retaining checker/compiler subtotals. It records the manifest's registration identity, digest, exact correction commit, tool identity, raw outputs, witnesses and holes. Every unmatched finding remains a false alarm under DP-1.1c. `cargo xtask docs` alone projects the latest archived H4 record into the enforcement map; every module cell remains not_run until CHG-007.

The first harness packet uses the accepted checker unchanged. Its failed run is retained before the checker repair determination and CHG-004.1 repair commit. Prior draft records remain archival observations of their named revisions; the final handoff will identify the replacement records explicitly.

## CHG-004.1 determination, before repair

[The first clean committed-fixture run](../../evidence/h4-crate/20260920T121342Z-3c6c758911ff-1462546.json) describes `3c6c758911ffa418fbe5b2678e7b538c36a69521`. It exits 1: **13/22 detected**, **10/11 unmatched findings**, two documented holes. R01 and all 11 legitimate cases pass; C04, C05, C08, C09, C10, C11, C13, C14 and C20 fail exact witness grading.

The raw reports show those checker violations, but their structured JSON omits registered `crate`, cycle `members`, `matched_rule`, conflicting-role values, `port` or `owner` fields. Add those facts at the producer from its graph/rules observations; never derive an answer from the corpus inside checker code. The separate CHG-004.1 commit applies this repair after this determination and failed record. C13's additional `adapter.foreign_core` warning is accurate and must remain an unmatched finding under DP-1.1c; it is not suppressed to obtain a green H4 result.
