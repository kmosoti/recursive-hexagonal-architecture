# CHG-008 (P-D) resume point

This is the handoff for packet P-D (CHG-008 to CHG-014, plan §8.1 P-D and §8.2 W8 to W10). It is written for whoever continues the work, and it names no particular model or agent. Update it, or delete it in the commit that closes P-D. The full history is in `docs/changes/CHG-008-growth.md` and `.rha/tasks/CHG-008-growth.toml`.

Written 2026-09-23 by the supervising session, after the previous Executor's run stopped at a Codex usage limit, at 05:12 on 2026-09-23. Nothing is lost:

- The branch `chg/008-growth` is pushed.
- Its head is `44ef02a`, a labelled work-in-progress snapshot of the uncommitted tree at the stop.

## Where the packet stands

| Stage | State | Evidence |
| --- | --- | --- |
| Stage 0: assurance prerequisites (renderer and sink contract gaps, the unchanged-page count, graph uniqueness precondition) | Done | CHG-008 change record, "Stage 0"; `evidence/CHG-008/stage-0/` |
| Stage 1: `adapter-json` and H3 | Done: 79 registered cases pass; the isolated core diff is empty; the H3 detection probe caught all three faulty renderers | BDR-0006; `docs/architecture/json-renderer-contract.md`; `docs/observations/h3-json.md`; `evidence/CHG-008/stage-1/` |
| Stage 2, feature 1: transclusion `![[Page#Section]]` | Done: gate passed (22cb415) | BDR-0007; decisions `transclusion-*`; `evidence/CHG-008/stage-2/transclusion-stage-*` |
| Stage 2, feature 2: §-references | Done: gate passed (f18661e) | `docs/architecture/section-reference-contract.md`; `evidence/CHG-008/stage-2/section-refs-stage-*` |
| Stage 2, feature 3: `[Rn]` citations | Done: gate passed at 0339267 after three review rounds | `docs/architecture/citation-contract.md`; `evidence/CHG-008/stage-2/citations-stage-*` |
| Stage 2, features 4 and 5 (callouts, tags and front matter) | Not started | plan W9 |
| Stage 3: mutation analysis (DP-2.2) | Not started | plan W10 |
| Review and PR | Not started | |

## Unfinished work at the snapshot (`44ef02a`)

`cargo check --workspace --all-targets` passes. `cargo test --workspace` gives 301 passing and 3 failing:

1. `xtask` `module_test_source_boundary`: two tests fail, `an_inner_file_test_attribute_cannot_grant_external_read_authority` and `a_root_inner_test_attribute_cannot_grant_external_read_authority`. These are the regression tests for an open bug.
   - **Background:** the full-workspace architecture check now lets test-only modules live outside their crate directory, because the transclusion graders share one loader through `#[path]`. The decision is `workspace-test-source-boundary`, specified in `docs/architecture/module-test-source-boundary.md`. The 12 controls in `xtask/tests/corpus/module-test-boundary/` were registered in `71a96ea` and all pass.
   - **The bug:** a separate review found that an inner `#![cfg(test)]` or `#![test]` attribute in a file grants that external-read authority. Only an explicit test declaration should grant it.
   - **The fix:** repair the authority tracking in `xtask/src/modules/extract.rs` and `xtask/src/modules/workspace.rs`. Test-edge classification must be kept, and the frozen expectations must not change.
2. `xtask` `graph::rules::tests::the_rules_file_of_this_repository_parses` fails.
   - **The cause:** the snapshot widens `rha-crates.toml` `[core] dev_allow` to `["proptest", "serde_json"]`, so that core tests can check their JSON oracles (decision `owner-local-json-oracles`). That test pins the list on purpose, so a widening is a visible change.
   - **The fix:** confirm the decision covers this protected edit and quotes its approval, then update the pin in `xtask/src/graph/rules.rs` with a §9.14 justification in the change record.

After these, run the transclusion stage gate:

- `cargo nextest run --workspace --all-features --profile ci`;
- `cargo xtask docs --check`;
- `cargo xtask architecture`;
- `cargo xtask scope --task CHG-008`;
- clippy;
- the change-spread record: expected against observed touched sets, in `docs/observations/change-spread.md`.

## Owed from outside this branch

P-C (PR 18) was merged at `74aeb1d` while this branch was being written. The previous Executor never learned of it, because the queued messages did not reach its session.

1. **Merge `origin/main` into `chg/008-growth`.** Never rebase. A dry run shows that only `docs/tasks/index.md` conflicts; regenerate it with `cargo xtask docs`. The extractor changes on main (CHG-007.2 to CHG-007.4) and this branch's changes to `extract.rs` combine without conflict. Even so, rerun `xtask/tests/module_review_threads.rs` and the module H4 afterwards.
2. **Write P-C's acceptance record next:** `.rha/acceptances/CHG-007.toml`, bound to merge `74aeb1d` and to the main CI run at that merge, run `35841881942`. Download its evidence with `gh run download` into `evidence/ci/`. It is kind `bootstrap_acceptance`, with `[[disposition.predicates]]` as in `.rha/acceptances/CHG-005.toml`. Carry two open P3 items from the Opus review of PR 18:
   - a function-local `mod` in the glue root that shares a component's name is attributed to that component;
   - a finding raised inside a block module has no `source_file` in its witness.
3. **Shrink the evidence records.** The module H4 records repeat the committed random expectations, at 6.5 MB or more each. Make the harness store digests and counts instead (plan M8). This was an open question in the PR 18 review.

## Standing rules that apply

- Merge rule (Kennedy, 2026-09-22): a PR merges only when an Opus 5.5 agent and a GPT-6 agent have each reviewed and approved the same head. The integrating session merges; the author never does.
- GPT-6 reviews use `codex exec -m gpt-6-sol -c model_reasoning_effort=medium -s read-only` (ledger DP-5.4, as amended 2026-09-23).
- Ask Kennedy before using `gpt-6-astra` for any role.
- The Codex account's usage limit resets on 2026-09-29 at 21:09.
- Decisions are taken under Kennedy's delegation: take the recommended or most correct option, record it in the task record and on the ledger, and quote the delegation. A protected edit needs a recorded justification and a `protected_scope` entry.
- Every graded corpus is registered before the code it grades runs (M7). A registered grading changes only through the ledger amendment procedure, disclosed. Nothing is reported passed that did not run. History is never rewritten.
- Held-out material stays Kennedy's (§9.14); record held-out runs `not_run`.
- Run clippy and the full lane before every push, and `cargo xtask docs` (it takes no `-q` flag) before `docs --check`.
- Tooling belongs in xtask's Rust. Throwaway scripts stay in `target/` and are not committed.

## Commands to verify the resume point

```text
git -C <worktree> log --oneline -1          # 44ef02a, or later
cargo test --workspace                      # 301 passed, 3 failed at 44ef02a (listed above)
cargo xtask architecture                    # the real workspace passes
git merge-tree --write-tree origin/main HEAD  # only docs/tasks/index.md conflicts
```
