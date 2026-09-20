# CHG-004: H4 crate-level harness

Task: [CHG-004](../../.rha/tasks/CHG-004-h4-crate-harness.toml). Plan: §6 and §8 W4.
Base: `140fdfb92d62d3481d95f3e2efc1d24a0206fcc4`, merge of PR 6.

## Required order and approval

1. `89166ed002448c3075f3fb02dcd2689e5a4752ca` records CHG-003 acceptance and archives the CI evidence for the accepted merge.
2. This commit changes EM-M03's cell association in the manifest to `cells = ["law3-d1", "law6-b3"]`, with its pinned test row and this task record.
3. Harness implementation follows those commits.

Kennedy approved the protected manifest edit in the working conversation on 2026-09-20: “I approved the EM-M03 cell edit on the manifest: cells = ["law3-d1", "law6-b3"].” Under §9.14 this corrects the test's cell attribution with explicit authority. Every other case and all grading values remain fixed (DP-1.1c).

Kennedy runs his held-out cases at acceptance (DP-1.1b); they are never read by the Executor. Accepted maturity remains his decision (DP-1.6).
