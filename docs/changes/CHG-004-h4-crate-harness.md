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
