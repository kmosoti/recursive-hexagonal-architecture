# CHG-003: `cargo xtask architecture`, the crate-graph checker

Task record: [`.rha/tasks/CHG-003-architecture-crate-level.toml`](../../.rha/tasks/CHG-003-architecture-crate-level.toml).
Plan: [§5 and §8 W3](../plan/IMPLEMENTATION-PLAN.md).
Base: `4be4a19`, the merge that accepted CHG-002.2.

## Intent

Implement the checker of plan §5 at crate level. It is the change that makes `L0.architecture` run, which has blocked the `Passed` predicate since CHG-000 — for three work items every acceptance record has carried the same sentence, that a required non-waivable check was `not_run`.

## Commit order, and why it is the order

The first two commits contain no checker code, and that is the point.

1. **`2ce0651`** — `.rha/acceptances/CHG-002.2.toml`. `.rha/policy.toml [acceptance]` makes the merge the acceptance and the record the first commit of the next work item.
2. **`b0b0636`** — [`.rha/decisions.toml`](../../.rha/decisions.toml) DP-1.1b, carrying Kennedy's commitment digest with `status = "decided"`. The row said the digest must be recorded *before any checker code exists*, "or it cannot show the cases predate the checker". After this commit no open ledger row names `CHG-003` in `blocks`.
3. **`7d07630`** — the task record and the `rha-crates.toml` schema.
4. **`665d84e`** — the checker.

`git log` shows the order. It is the evidence, not this paragraph.

## What the checker is

**[`xtask/src/metadata.rs`](../../xtask/src/metadata.rs)** runs `cargo metadata --format-version 1 --no-deps --offline` and builds the graph from its fields and nothing else. Plan §8 W3 requires that renamed, optional, target-conditional and workspace-inherited dependencies be resolved from metadata and **never** from `Cargo.toml` text. Corpus C15–C18 are four spellings of one edge written to fail a checker that parses the text; cargo has already resolved all four, so there is nothing to special-case.

**[`xtask/src/graph/`](../../xtask/src/graph/)** — `model` is the graph, `classify` decides roles, `rules` is the rules file, `check` evaluates, `report` renders.

Classification is **total** (§11.2): metadata role, then prefix, then the tools and harness lists, else `class.unclassified` — an error, never a default role, because a default would let an unclassified crate pass every direction rule silently. The prefix is matched with `starts_with`, so corpus L11's `planner-adapter-utils` is not an adapter.

Sixteen rules, each with a witness naming the edge, the manifest path and the declaring section: `class.unclassified`, `class.prefix_role_conflict`, `dir.core_to_adapter`, `dir.core_to_app`, `dir.non_root_to_adapter`, `dir.tool_depended_on`, `effect.core_disallowed_dependency`, `effect.core_disallowed_dev_dependency`, `effect.core_build_script`, `effect.core_clippy_template` (wired in from CHG-001), `adapter.missing_port_owner`, `adapter.port_owner_wrong_kind`, `adapter.foreign_core` (warning), `meta.unknown_port_owner`, `forbidden.edge`, `graph.cycle`.

Exit codes as §5 fixes them: `0` no errors, `1` error findings, `2` usage or configuration, `3` environment failure. Three matters most: a `cargo metadata` failure writes a report with `error_class: tool_error` and **is never a pass**. A checker that could not run is not a checker that found nothing.

**A rule that cannot be evaluated says so.** `transitive.*` needs resolved metadata and is a stated limitation in `--no-deps` mode; `effect.core_clippy_template` examined nothing because no crate is classified core yet, and says that rather than letting an empty findings list read as a pass over a core crate.

## The corpus caught a real bug

L07 seeds a core's **dev**-dependency on an adapter and expects `no_alarm`. I wrote the dev-kind exclusion into `dir.non_root_to_adapter`, which is where plan §5 mentions it, and missed that `dir.core_to_adapter` is checked first and returns early — so the edge still failed. Reading the plan, the implementation looked right.

The expectation was fixed in W2, before any of this code existed, so when the checker disagreed the checker was the thing in question. That is what pre-registration is for, and it is the first time in this program it has paid out against code rather than against documents.

The exclusion now covers D1 as well as D5, and the edge is listed under `harness_edges`: silence and a listed fact are different outcomes.

## Deliberate changes to existing files

**`xtask/Cargo.toml` drops `[package.metadata.rha] role = "tool"`.** It is classified by the `tools` list in `rha-crates.toml`, which plan §5 makes the owner of that fact and which plan §8 W5's acceptance expects as `xtask: tool (list)`. Stating it in both places meant metadata silently won, so removing `xtask` from the list would have had no effect — one fact, two owners, and the loser silent (§11.7.1). Not a protected surface.

**A CHG-002 test is narrowed, under §9.14.** `no_fixture_workspace_and_no_checker_is_committed_yet` also asserted that `xtask/src/graph` did not exist and that `architecture.rs` still contained `EXIT_NOT_RUN`. Both became false here. This is **the approved requirement changing**, not the test becoming inconvenient: CHG-002's non-goal was that *that item* must not implement the checker, and the ordering it guarded is now a historical fact — the corpus merged at `15d916a` and every line of the checker is newer. The fixture half still runs, because CHG-004 has not happened. The test is renamed to what it now checks, and its doc comment records what was removed and why.

**`rha-crates.toml`** receives the plan §5 schema with **empty** allow-lists, pre-approved by Kennedy. Empty is not a placeholder: it is the strictest available statement, that no core crate may depend on anything external. Filling the lists from §5's example would pre-authorize dependencies nothing has asked for, and §11.1 wants each justified when it arrives.

## Tests

26 tests on synthetic crate graphs in [`xtask/tests/architecture_rules.rs`](../../xtask/tests/architecture_rules.rs), one or more per rule, each named after the corpus case it mirrors so that a disagreement in W4 reads as a disagreement rather than as two unrelated failures.

They are synthetic **on purpose**. The checker must not read `xtask/tests/corpus/manifest.toml`: a checker that reads the cases it is judged on can satisfy one without implementing the rule, which is what pre-registration exists to prevent. A test asserts that no checker module names the manifest. Two other places in xtask do name it legitimately and are out of that test's scope — `corpus.rs` holds the manifest's types for CHG-004's harness, and `evidence/mod.rs` lists it among the fixtures whose digests go into a record, which is the opposite of consulting the answers.

## Repair attempts (§11.7.8)

1. **`L0.typos` failed at `102abc0`.** A test function in `xtask/src/graph/rules.rs` was named with a misspelled plural of "repository". The tool was right: unlike CHG-002's hex digests, this was a word, and it was wrong.

   The first repair renamed the function and carried the misspelled word into the new name, so the check failed again on the same line — a rename has to change the *word*, not the identifier around it. Renamed to `the_rules_file_of_this_repository_parses`.

   `_typos.toml` was not touched. A real misspelling is not a false positive, and adding it to `extend-words` to make a check pass is exactly the shape §9.14 warns about. This paragraph also avoids quoting the misspelling, because CHG-002 learned that documenting one puts it back in the tree.

## Evidence

**Local**: `evidence/CHG-003/20260920T093725Z-53563857be6c.json` at `5356385`.
**CI**: run [35502859792](https://github.com/kmosoti/recursive-hexagonal-architecture/actions/runs/35502859792), copied as [`evidence/ci/35502859792.json`](../../evidence/ci/35502859792.json), subject `329dcc4d`.

Both agree, on two machines:

| Check | Outcome |
| --- | --- |
| `L0.fmt`, `L0.clippy`, `L0.nextest`, `L0.doctest`, `L0.deny`, `L0.machete`, `L0.typos` | passed |
| **`L0.architecture`** | **passed** |

108 tests selected. `cargo xtask architecture` on the CI runner reports `{"crates": 1, "errors": 0, "outcome": "passed", "warnings": 0}`, the same as locally.

| Predicate | Holds | Why |
| --- | --- | --- |
| Authentic | **false** | no trusted producers in `.rha/policy.toml` until CHG-019 and CHG-020 |
| Applicable | true | the candidate's policy equals the base's; this change edits no policy |
| Complete | true | every L0 check appears with a unique id |
| **Passed** | **true** | **first time in this repository** |

Eligibility remains `blocked`, on `Authentic` alone. `.rha/policy.toml [acceptance.bootstrap]` records `passed_unsatisfiable_until = "CHG-003: L0.architecture is not implemented"`; that clause expires with this merge, and its `expires` condition now waits only on CHG-020.

## Acceptance concerns

1. **`Passed` can hold; `Authentic` still cannot.** With `L0.architecture` running, seven of the eight blockers on eligibility are gone. `Authentic` needs trusted producers in `.rha/policy.toml`, which arrive with CHG-019 and CHG-020, so no record is Eligible yet and every acceptance stays a controlled transition under `[acceptance.bootstrap]`.
2. **The checker has one crate to check.** `xtask` is the only member, and it is a tool, so most rules examined nothing here. They are exercised by unit tests on synthetic graphs, and by the corpus in CHG-004. Nothing runs them against real product crates until CHG-005.
3. **`effect.core_clippy_template` examined nothing**, because no crate is classified core. It is wired in and will fire from CHG-005; until then its limitation line says so, and a reader should not read its silence as a pass.
4. **The cycle rule may be unreachable in practice.** Cargo rejects a package cycle while resolving, so a cyclic workspace can fail before the checker sees it. Corpus C05 requires this rule's own finding — CHG-002.1 removed the `alternative` that would have let a cargo error count — so W4 will show whether the case reaches the checker at all.
5. **`--transitive` is accepted and does nothing** unless `[transitive] enabled` is true in the rules file. The flag prints a line saying so rather than silently widening or silently ignoring; turning it on is the rules file's decision, not the command line's.
6. **Glob matching is one trailing `*`.** Plan §5's examples need no more, and a full glob engine would be a dependency and a second syntax. A rule needing more will say so.
