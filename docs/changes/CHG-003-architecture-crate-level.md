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

### Review findings (Codex on pull request 6, reviewed revision `776fa83`), determinations

The rule applied, adopted on 2026-09-20 after Codex's audit of this repository's review history: a finding becomes a `CHG-00n.k` commit only when it names a concrete failure on the reviewed revision; bookkeeping findings are batched into one determination; a resolved thread reopens only on a new failure mode. All three findings here name a failure, so all three are repaired, in one commit.

1. **P1, a registry package with a member's name was classified as the member. Confirmed.** `metadata::build` decided membership by comparing the dependency's package name with the member names. A core depending on a registry crate named like a member (crates.io has a `graph`; this repository will have one) got a `Target::Member` edge, and `effect_rules` skips member targets, so the dependency escaped `[core] allow`. Change: membership is decided by location. `cargo metadata` reports a path dependency with `path` set and `source` null; a dependency is a member only when its `path` is a member's manifest directory. A registry or git package, and a path package outside the workspace, are external whatever they are called, which also covers corpus C19. Result: a unit test seeds both shapes from a core and asserts both edges are external.
2. **P1, `[transitive] enabled = true` removed the limitation while nothing evaluated the rule. Confirmed.** Metadata is always read with `--no-deps` and no transitive evaluator exists, so a rules file that enabled it got a clean summary and exit 0 for a check that was requested and never performed: on an EM-C01 workspace, a silent pass over the very hole the case registers. Change: `architecture::run` refuses `enabled = true` with exit 2, the plan §5 code for a configuration this version cannot honour, before `cargo metadata` runs; the lane records exit 2 as `failed` with `error_class: config_error`, never `passed`. `check()` states the limitation in both settings, so an in-process caller is told too. `--transitive` alone prints a notice and continues, as before. Result: a test with a rules file enabling transitive mode and no `Cargo.toml` gets 2, not the environment failure 3 it would get if anything had run first; the in-process limitation is asserted with `enabled = true`.
3. **P2, `foreign_core_dependency` accepted any string and read a misspelling as warn. Confirmed.** `deny_unknown_fields` guards this file's keys, not its values; a value such as `"fatal"` parsed and meant warn, so a rules file meant to be fatal produced warnings and exit 0. Change: the value is a closed enum, `warn | error`; anything else is a parse error and exit 2. Result: a unit test asserts `error` is fatal, `warn` is not, and two other spellings are rejected.

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
5. **`--transitive` is accepted and does nothing, and `[transitive] enabled = true` is refused** with exit 2 (CHG-003.1). This version has no transitive evaluator; the flag exists so the L2 interface is stable, and the rules-file setting is refused rather than run as if it had been honoured. The limitation line names the rule in both settings.
6. **Glob matching is one trailing `*`.** Plan §5's examples need no more, and a full glob engine would be a dependency and a second syntax. A rule needing more will say so.
7. **CHG-000 and CHG-001 were merged with no completed automated review.** Pull requests 1 and 2 received review usage-limit messages and zero completed passes (Codex's audit of the review history, 2026-09-20). The absence of review findings in those two acceptance records is not a clean review and should not be read as one. Nothing here changes those records.
8. **The `controlled_transition` label is borrowed.** Spec §11.5 uses the term for a policy migration; `[acceptance.bootstrap]` applies it to every acceptance in the bootstrap period, including ordinary changes that touch no policy, such as the one this item's first commit recorded. A distinct kind is proposed in `docs/proposals/spec-v0.11.md` row 8 and needs a policy edit that only Kennedy makes; until then each record's `basis` says what the acceptance is, and the label stays.
