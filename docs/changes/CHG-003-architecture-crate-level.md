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

Tests on synthetic crate graphs in [`xtask/tests/architecture_rules.rs`](../../xtask/tests/architecture_rules.rs), one or more per rule, each named after the corpus case it mirrors so that a disagreement in W4 reads as a disagreement rather than as two unrelated failures.

They are synthetic **on purpose**. The checker must not read `xtask/tests/corpus/manifest.toml`: a checker that reads the cases it is judged on can satisfy one without implementing the rule, which is what pre-registration exists to prevent. A test asserts that no checker module names the manifest. Two other places in xtask do name it legitimately and are out of that test's scope — `corpus.rs` holds the manifest's types for CHG-004's harness, and `evidence/mod.rs` lists it among the fixtures whose digests go into a record, which is the opposite of consulting the answers.

## Repair attempts (§11.7.8)

1. **`L0.typos` failed at `102abc0`.** A test function in `xtask/src/graph/rules.rs` was named with a misspelled plural of "repository". The tool was right: unlike CHG-002's hex digests, this was a word, and it was wrong.

   The first repair renamed the function and carried the misspelled word into the new name, so the check failed again on the same line — a rename has to change the *word*, not the identifier around it. Renamed to `the_rules_file_of_this_repository_parses`.

   `_typos.toml` was not touched. A real misspelling is not a false positive, and adding it to `extend-words` to make a check pass is exactly the shape §9.14 warns about. This paragraph also avoids quoting the misspelling, because CHG-002 learned that documenting one puts it back in the tree.

2. **A lane ran on an uncommitted tree during CHG-003.2.** The consolidation was staged as two commits, but the `git add` for each named a path that `git rm` had already removed, so the shell skipped both commits and the lane that followed ran on the working tree. Hypothesis: a shell ordering error, nothing about the code. Discriminating check: `git log` showed no new commit and the record is named `-dirty`. The record, `evidence/CHG-003/20260920T110910Z-920cca6b07b7-dirty.json`, is what CONTRIBUTING says a dirty record is, all eight checks `passed` over the not-yet-committed consolidation, and it is kept. Change: the two commits were made, then this record committed on its own, then the lane re-run on the clean tree. Result: the CHG-003.2 record under Evidence.

### Review findings (Codex on pull request 6, reviewed revision `776fa83`), determinations

The rule applied, adopted on 2026-09-20 after Codex's audit of this repository's review history: a finding becomes a `CHG-00n.k` commit only when it names a concrete failure on the reviewed revision; bookkeeping findings are batched into one determination; a resolved thread reopens only on a new failure mode. All three findings here name a failure, so all three are repaired, in one commit.

1. **P1, a registry package with a member's name was classified as the member. Confirmed.** `metadata::build` decided membership by comparing the dependency's package name with the member names. A core depending on a registry crate named like a member (crates.io has a `graph`; this repository will have one) got a `Target::Member` edge, and `effect_rules` skips member targets, so the dependency escaped `[core] allow`. Change: membership is decided by location. `cargo metadata` reports a path dependency with `path` set and `source` null; a dependency is a member only when its `path` is a member's manifest directory. A registry or git package, and a path package outside the workspace, are external whatever they are called, which also covers corpus C19. Result: a unit test seeds both shapes from a core and asserts both edges are external.
2. **P1, `[transitive] enabled = true` removed the limitation while nothing evaluated the rule. Confirmed.** Metadata is always read with `--no-deps` and no transitive evaluator exists, so a rules file that enabled it got a clean summary and exit 0 for a check that was requested and never performed: on an EM-C01 workspace, a silent pass over the very hole the case registers. Change: `architecture::run` refuses `enabled = true` with exit 2, the plan §5 code for a configuration this version cannot honour, before `cargo metadata` runs; the lane records exit 2 as `failed` with `error_class: config_error`, never `passed`. `check()` states the limitation in both settings, so an in-process caller is told too. `--transitive` alone prints a notice and continues, as before. Result: a test with a rules file enabling transitive mode and no `Cargo.toml` gets 2, not the environment failure 3 it would get if anything had run first; the in-process limitation is asserted with `enabled = true`.
3. **P2, `foreign_core_dependency` accepted any string and read a misspelling as warn. Confirmed.** `deny_unknown_fields` guards this file's keys, not its values; a value such as `"fatal"` parsed and meant warn, so a rules file meant to be fatal produced warnings and exit 0. Change: the value is a closed enum, `warn | error`; anything else is a parse error and exit 2. Result: a unit test asserts `error` is fatal, `warn` is not, and two other spellings are rejected.

### CHG-003.1 follow-up on reviewed revision `4e33733`

Kennedy requested repairs for the remaining P1/P2 findings and authoring
guidance based on this review history. Seven open threads describe four
distinct defects; duplicate comments on the same revision are one repair
obligation. The three earlier CHG-003.1 repairs above are unchanged history.

| Finding | Discriminating observation before the repair | Required result |
| --- | --- | --- |
| [Port-owner identity](https://github.com/kmosoti/recursive-hexagonal-architecture/pull/6#discussion_r4056688908), repeated in `4056690736` | An adapter claims a member's port but depends on an external package with the same name; the CLI exits 0 with no findings. | Only a normal dependency on the actual member satisfies the declaration. A valid normal edge must work even when a dev edge occurs first. |
| [Malformed metadata](https://github.com/kmosoti/recursive-hexagonal-architecture/pull/6#discussion_r4056688909) | A prefixed adapter with a string-valued `implements` table entry exits 0 because the declaration is discarded. | Missing optional metadata remains valid; present malformed metadata reports its manifest and exits 2 as a configuration error. Cargo execution failures remain exit 3. |
| [Unsupported schema](https://github.com/kmosoti/recursive-hexagonal-architecture/pull/6#discussion_r4056688910), repeated in `4056690738` | Rules declaring schema version 2 produce a clean report under version 1 semantics. | Reject unsupported versions at the parser, before loading Cargo metadata. |
| [Checker identity](https://github.com/kmosoti/recursive-hexagonal-architecture/pull/6#discussion_r4056688912), repeated in `4056690740` | Successful JSON has only the tool name and package version; `tool.git_rev` is absent. | Success and failure reports identify the checker built from this source, independently of the checked workspace, and disclose dirty or unavailable identity. |

The baseline CLI was built from `4e33733`; all three invalid-input fixtures
exited 0, and the fourth observation confirmed the missing field. These
small external workspaces are regression probes, not the deferred H4 corpus
runner. The same probes after repair return exit 1 for the wrong member and
exit 2 for malformed metadata and the unsupported schema. A valid workspace
still exits 0; a missing Cargo manifest exits 3 with `tool_error`. The compiled
checker identity is shared by success and failure reports, while a failed
load leaves the unknown subject root null. The observations are retained in
[review-followup-probes.txt](../../evidence/CHG-003/review-followup-probes.txt).

The port check now prefers any normal edge to the actual member, regardless
of dev/build edge order. Metadata errors preserve their configuration or
transport classification, and the rules parser rejects unsupported versions.
Valid `composite` metadata and the declared `--transitive` flag-only behavior
remain supported. The old unit test that treated malformed metadata as absent
was corrected under §9.14: absence and invalid declarations have different
contracts, now checked separately. No corpus expectation was changed.

A standard-library-only build script in the `xtask` tool captures Git revision
and dirty source status. Its Git reads occur when building repository tooling,
not in a core crate; it adds no dependency. Resolved worktree HEAD/branch/index
paths refresh that identity without scanning the shared Git directory. A
binary keeps its compiled identity when used on another workspace.

Independent closure approved the corrected tests, exit/report semantics,
identity watches and guide changes. The package's targeted test run passed;
the final lane result is recorded below.

The authoring changes address the demonstrated causes: distinguish absence
from malformed input; preserve identity through downstream decisions; test
real input boundaries alongside synthetic graphs; and assert nested report
contracts. They do not add another approval stage or a fixed review quota.

### CHG-003.2: consolidation, determinations

Authority: Kennedy's instruction of 2026-09-20, after disabling automated review, to bring this PR to a compliant and efficient state and fix the churn according to the spec. Task record decision `consolidation-chg-003.2`.

1. **The follow-up's four repairs stand.** Port ownership requires the member, malformed metadata is a configuration error, `schema_version` must be 1, and every report identifies the checker. Their tests stay.
2. **The build script is replaced by run-time identity. Determined against §11.7.1 and §6.8.** The tool's revision had two owners with two definitions of dirty: `xtask/build.rs` at build time over the tool's own paths, and `xtask ci` at run time over the whole tree for the evidence record's `producer`. One lane run could say `tool_from_dirty_tree: true` in the evidence record and `git_dirty: false` in the architecture report for the same binary. §11.7.1 gives a fact one owner, and a build script that reads git is the build-time ambient effect the checker forbids in core crates (`effect.core_build_script`), which a tool should not model. Change: `util::git_identity` reads `HEAD` and `git status --porcelain --untracked-files=all` at run time; `report::tool_identity(root)` uses it for success and failure reports alike. `cargo xtask` rebuilds the binary from the working tree before every run, so the running checker is that checkout, and `root` is the tool's own workspace, never the subject under `--manifest-path`. The 153-line build script is deleted, and xtask no longer rebuilds on every `git add`. Result: the follow-up's CLI test compares the report's `tool` object with the same call in-process, and a unit test covers a checkout and a directory that is not one.
3. **The PR template is restored to its state at `4e33733`.** `.github/**` is a protected surface and no recorded approval names it; the follow-up's decision cites a request for "related agentic files", which does not reach a protected path. The line it added is guidance CONTRIBUTING.md owns.
4. **`docs/orchestration-log.md` is removed.** It journals the executor's internal worker roles and is not an artifact of §11.7.11 or plan §3.4; the wiki would render it as research content. Its provenance facts are on the task record's `[[provenance.sessions]]`, and the text remains in history at `920cca6`.
5. **Guidance is owned once.** AGENTS.md returns to its entry-point shape; CONTRIBUTING.md owns the workflow rules, including the follow-up's authoring paragraph and its hashing rule; `xtask/AGENTS.md` owns the scoped rules, with "workers" and "coordinator" replaced by repository terms. The CONTRIBUTING bullet that allowed a final lane without a record is replaced by the ordering this repository uses: docs, then lane, then commit; a record is taken on a clean tree and committed in the next commit; commits that only add a record or an index need no record of their own, since the CI record for the pushed head covers them.
6. **Older threads, batched as bookkeeping.** Pull request 3: the CHG-001 task record's provenance now names the W1.4 permission (`.rha/acceptances/CHG-001.toml`, approved in decision W1.4), the CHG-001.4 instruction, and the CHG-001.4 session; the W1.4 evidence record copied the earlier text and is not rewritten. Pull request 4: the CHG-002 task record's module-level totals read 17 violations, 6 legitimate, 3 expected misses and X-M01, which is what the manifest holds; its change record's evidence section lists all five local records and the three later CI runs; EM-M03's cell needs a protected-manifest edit and waits for Kennedy (acceptance concern 9). Pull request 5: the digest finding is D-002.2.1; the repeated bootstrap objection names no new failure mode beyond the one `[acceptance.bootstrap]` discloses and bounds, and the label question is proposal row 8, so nothing changes.
7. **Churn.** The lane ran once per settled packet in this round, not once per commit. The final head carries one clean local record and one CI record; the follow-up's dirty snapshot record is kept as what it is.

### CHG-003.3: two findings from the last automated pass on `920cca6`, determinations

Posted at 10:43 UTC, before the review was disabled, and found after the CHG-003.2 threads were closed. Both reproduced on a temporary workspace before repair.

1. **P1, `effect.core_clippy_template` had never examined a crate. Confirmed.** `architecture::run` handed the rule the graph as loaded, whose roles are all `None`: `check::check` classifies a clone and returns only the outcome. A workspace with one crate declaring `role = "core"` and no `clippy.toml` exited 0 and reported the limitation "no crate is classified core". Change: the rule takes its cores from `outcome.classification`, which the check recorded with each crate's role and manifest path. Result: the same workspace exits 1 with two `effect.core_clippy_template` findings, no template and no crate-root forbid; with the template copied in and the forbid line present it exits 0. CLI regression `a_core_without_the_clippy_template_is_reported_and_with_it_is_not`. This repository has no core crate, so its own reports were not wrong; the mechanism was, and CHG-005's cores would have gone unexamined.
2. **P2, an unknown `role:` selector was a rule that could never match. Confirmed.** `from = "role:cor"` parsed, and the check exited 0 with no finding. Change: `Rules::parse` rejects a `role:` selector that names none of the five roles, exit 2 `config_error`, the same closure at parse time as CHG-003.1's `foreign_core_dependency`. Unit test `a_forbidden_selector_with_an_unknown_role_is_a_parse_error`.

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

**CHG-003.1, local**: `evidence/CHG-003/20260920T100013Z-1cbc47ec97f2.json` at `1cbc47e`, the repaired revision, on a clean tree: all eight checks `passed`, 112 tests selected, eligibility `blocked` on `Authentic` alone.
**CHG-003.1, CI**: run [35503860516](https://github.com/kmosoti/recursive-hexagonal-architecture/actions/runs/35503860516) on the pushed head `b4adcf2`, copied as [`evidence/ci/35503860516.json`](../../evidence/ci/35503860516.json), subject `1731b0d6`: all eight `passed`, 112 tests.

**CHG-003.1 follow-up, local snapshot**: [20260920T103310Z-4e337339c121-dirty.json](../../evidence/CHG-003/20260920T103310Z-4e337339c121-dirty.json)
records all eight L0 checks `passed`, with 121 tests selected. It identifies
an explicitly dirty snapshot based on `4e337339c121`;
it does not claim to cover later evidence/index edits. After those edits are
committed, the final lane is run again without `--record`, keeping its clean
candidate result in `target/rha/evidence.json` without another archive cycle.

**CHG-003.1 follow-up, CI**: run [35505682590](https://github.com/kmosoti/recursive-hexagonal-architecture/actions/runs/35505682590) on the pushed head `920cca6`, copied as [`evidence/ci/35505682590.json`](../../evidence/ci/35505682590.json), subject `de02534d`: all eight `passed`, 121 tests. `920cca6` has no committed local record: the snapshot record above carries `snapshot_tree` `1fb5abcd…`, which is not that commit's tree, `041fcd1d…` (CHG-003.2 added this record and this note).

**CHG-003.2, local**: `evidence/CHG-003/20260920T111037Z-956707e0b1da.json` at `956707e`, the consolidated revision, on a clean tree: all eight `passed`, 122 tests, eligibility `blocked` on `Authentic` alone.
**CHG-003.2, CI**: run [35507106318](https://github.com/kmosoti/recursive-hexagonal-architecture/actions/runs/35507106318) on the pushed head `26e7519`, copied as [`evidence/ci/35507106318.json`](../../evidence/ci/35507106318.json), subject `7a419405`: all eight `passed`, 122 tests. This local record and this CI record are the evidence for the code this PR proposes; every earlier record is kept as what it was.

**CHG-003.3, local**: `evidence/CHG-003/20260920T111702Z-a925cfcda441.json` at `a925cfc`, on a clean tree: all eight `passed`, 124 tests, eligibility `blocked` on `Authentic` alone. With its CI record below, this is the evidence for the code this PR proposes; the CHG-003.2 records describe the revision before the two repairs.

Eligibility remains `blocked`, on `Authentic` alone. `.rha/policy.toml [acceptance.bootstrap]` records `passed_unsatisfiable_until = "CHG-003: L0.architecture is not implemented"`; that clause expires with this merge, and its `expires` condition now waits only on CHG-020.

## Acceptance concerns

1. **`Passed` can hold; `Authentic` still cannot.** With `L0.architecture` running, seven of the eight blockers on eligibility are gone. `Authentic` needs trusted producers in `.rha/policy.toml`, which arrive with CHG-019 and CHG-020, so no record is Eligible yet and every acceptance stays a controlled transition under `[acceptance.bootstrap]`.
2. **The checker has one crate to check.** `xtask` is the only member, and it is a tool, so most rules examined nothing here. They are exercised by unit tests on synthetic graphs, and by the corpus in CHG-004. Nothing runs them against real product crates until CHG-005.
3. **`effect.core_clippy_template` examined nothing**, because no crate is classified core. It is wired in and will fire from CHG-005; until then its limitation line says so, and a reader should not read its silence as a pass. CHG-003.3 repaired the wiring itself: before it, the rule was given the unclassified graph and would not have fired for any workspace.
4. **The cycle rule may be unreachable in practice.** Cargo rejects a package cycle while resolving, so a cyclic workspace can fail before the checker sees it. Corpus C05 requires this rule's own finding — CHG-002.1 removed the `alternative` that would have let a cargo error count — so W4 will show whether the case reaches the checker at all.
5. **`--transitive` is accepted and does nothing, and `[transitive] enabled = true` is refused** with exit 2 (CHG-003.1). This version has no transitive evaluator; the flag exists so the L2 interface is stable, and the rules-file setting is refused rather than run as if it had been honoured. The limitation line names the rule in both settings.
6. **Glob matching is one trailing `*`.** Plan §5's examples need no more, and a full glob engine would be a dependency and a second syntax. A rule needing more will say so.
7. **CHG-000 and CHG-001 were merged with no completed automated review.** Pull requests 1 and 2 received review usage-limit messages and zero completed passes (Codex's audit of the review history, 2026-09-20). The absence of review findings in those two acceptance records is not a clean review and should not be read as one. Nothing here changes those records.
8. **The `controlled_transition` label is borrowed.** Spec §11.5 uses the term for a policy migration; `[acceptance.bootstrap]` applies it to every acceptance in the bootstrap period, including ordinary changes that touch no policy, such as the one this item's first commit recorded. A distinct kind is proposed in `docs/proposals/spec-v0.11.md` row 8 and needs a policy edit that only Kennedy makes; until then each record's `basis` says what the acceptance is, and the label stays.
9. **EM-M03's cell.** `xtask/tests/corpus/manifest.toml` assigns EM-M03 only `law6-b3`, while its own hole describes a Law 3/D1 extraction miss (pull request 4 review finding, open at the time). Proposed edit, for your approval as a pre-registration correction made before any module checker exists: `cells = ["law3-d1", "law6-b3"]`. Until approved the manifest stands, and an observed miss on EM-M03 would be charged to the no-cycles cell alone.
