# CHG-002: Pre-registered H4 corpus manifest

Task record: [`.rha/tasks/CHG-002-corpus-pre-registration.toml`](../../.rha/tasks/CHG-002-corpus-pre-registration.toml).
Plan: [§6 and §8 W2](../plan/IMPLEMENTATION-PLAN.md).
Base: `44b1e44`, the merge commit that accepted CHG-001.

## Intent and scope

Commit the corpus that judges the crate-graph checker **before the checker exists**. Spec §17 fixes cases and thresholds before data; §9.14 forbids weakening a test quietly. Both are easy to say and easy to lose: once a checker runs, every case it fails looks like a case that was written wrong. Pre-registration removes that option by putting the expectations on the record first, and the merge commit of this change is the identity every H4 evidence record cites.

Sixty-two cases, transcribed from plan §6:

| Level | detect | no_alarm | expected_miss | reference | total |
| --- | --- | --- | --- | --- | --- |
| Crate | 22 (21 for the checker, plus R01 for rustc) | 11 | 2 | – | **35** |
| Module | 17 | 6 | 3 | 1 | **27** |

The module cases are registered here and built in CHG-007; only their ids, rules, cells and expectations are fixed now, which is all pre-registration requires.

**Non-goals, each checked by a test.** No checker code: `xtask/src/architecture.rs` is still the stub that reports `not_run`, and no `xtask/src/graph` module exists. No fixture workspaces under `xtask/tests/corpus/crate/` or `module/`. The last test in [`xtask/tests/corpus_manifest.rs`](../../xtask/tests/corpus_manifest.rs) asserts all three, so a later change cannot quietly turn this item into W3.

## Deltas

**[`xtask/tests/corpus/manifest.toml`](../../xtask/tests/corpus/manifest.toml)** — the corpus. Each case carries `id`, `level`, `generation`, `expected`, `rule`, `cell`, `seeded`, and a `witness`.

`cell` names a row of spec §4.1's enforcement map, so a miss downgrades a named cell rather than an unnamed feeling. The manifest adds one id the spec's table does not have, `classification`, with a comment saying why: total classification (§11.2) is not a §4.1 row but the precondition for every row, and a member the checker cannot classify is a hole in all of them at once. C10, C11 and C20 sit there.

`witness` states what a finding must name. A rule id alone would score C01 for a checker that reported `dir.core_to_adapter` about the wrong pair of crates, so C01's witness is `{ rule, from = "core-a", to = "adapter-x" }`, C20's names a crate and a port, and R01's names an error code.

**Generated, not committed.** `generation = "declared"` on all 35 crate-level cases means CHG-004 writes each fixture workspace from this file: `crates` gives the members, their `role` metadata, and their dependencies with `kind`, `source`, `version`, `path`, `target`, `optional`, `rename` and `inherit`. This is the CHG-001.1 decision applied before the drift exists — a committed fixture is a copy of the file that owns it, and the copy goes stale silently.

The 27 module cases are `generation = "authored"`: their seeded construct *is* Rust syntax (`impl crate::ordering::Trait for X`, a macro body, an `include!`), and encoding that in TOML would be a worse copy than the Rust. CHG-007 writes them.

**[`xtask/src/corpus.rs`](../../xtask/src/corpus.rs)** — the manifest's types and its validation. No runner. `Manifest::defects()` reports a repeated id, an unknown cell, a detection with no rule or no witness, an expected miss citing no §4.1 hole, a declared case with no crates, and a dependency on a crate the generator could not write. `#[serde(deny_unknown_fields)]` throughout, so a typo in a field name is an error rather than a case silently missing that field.

**[`xtask/tests/corpus_manifest.rs`](../../xtask/tests/corpus_manifest.rs)** — nine tests pinning the manifest to plan §6: the id set, the outcome counts, every rule id drawn from the plan's list, no defects, every crate-level case generatable, the four dependency shapes of C15–C18 each actually declared, C10's absent metadata distinguished from C11's conflicting metadata, and the grading rules stating what a disagreement does.

The plan's id list is transcribed into the test a second time on purpose. Reading it out of the manifest would make the test agree with whatever the manifest says.

**[`.rha/policy.toml`](../../.rha/policy.toml)** — one entry adding the manifest to `[surface] protected`, pre-approved by Kennedy. A corpus that is not protected can have an expectation edited after the fact, which is the one thing pre-registration exists to prevent.

## What the cases are for

Four groups do work beyond covering a rule.

**C15–C18 are one violation, four times.** `core-a → adapter-x` hidden behind a target `cfg`, an `optional`, a rename, and a workspace inherit. They exist to fail a checker that parses `Cargo.toml` text instead of reading `cargo metadata`'s resolved fields — the shortcut W3 will be tempted by. C17 additionally requires the finding to name the package `adapter-x`, not the rename `ax`.

**L07 and M11 expect a report that is not an alarm.** `core-a`'s dev-dependency on `adapter-x` is legitimate — a crate's own tests are a harness — but it must still be *listed*, under `harness_edges`. Their witness carries `listed_as` rather than `rule`, and `[grading].no_alarm_scope` says a run that raises it as a finding fails the case. Silence and a listed fact are different outcomes.

**L11 guards the checker against itself.** `planner-adapter-utils` declares `role = "core"` and contains the string `adapter-` without starting with it. A checker matching the prefix as a substring reports C11's conflict here and fails.

**EM-C02 records which mechanism covers a cell, not a failure.** `core-a` calls `std::fs::read` and declares no dependency: invisible to the crate graph, and caught by CHG-001's Clippy deny list at another level in another lane. Registering it keeps the enforcement map honest about where the coverage actually comes from.

## Grading, and what is still open

`[grading]` is what stops H4 grading itself generously. Three values are the Executor's proposal and are **open at DP-1.1**:

- `detection_requires` — the rule id plus every witness key naming a crate, port, module path or error code; `note`, `alternative` and `applies_when` are prose and are not matched.
- `extra_findings` — the case counts as detected, and every unmatched finding on that fixture is counted in the run's `false_alarm` total. Recall and precision are scored separately so an over-eager rule is neither hidden by a case it gets right nor punished twice for one defect.
- `no_alarm_scope` — any finding on a legitimate fixture fails the case, whatever its rule, except a report carrying `listed_as`.

Two more are not open, because they follow from §17 and §9.14, and both tests assert their wording: a `detect` case that is not detected fails the run and downgrades the §4.1 cell, and **never edits this file**; an `expected_miss` that *is* detected is escalated, not silently re-registered.

## Evidence

Local: `evidence/CHG-002/`. CI: `evidence/ci/`. Outcomes are listed in the pull request and in the item report, including `L0.architecture`, which is `not_run` and is reported as `not_run`.

**The policy changed again, so Applicable is false again.** Adding the manifest to the protected list moves the policy digest from `sha256:dc2ef7e0…` to `sha256:5497c29e…`, and a record is Applicable only where the candidate's policy equals the base revision's (§11.7.6). This is the second controlled transition in this repository; the first, CHG-001's, closed the moment it merged. Spec §11.5 makes it yours to accept.

### Repair attempts (§11.7.8)

1. **`L0.fmt` failed at `24ad160`.** The first lane run on this branch reported `L0.fmt failed: exited with 1`. Hypothesis: `xtask/src/corpus.rs` and `xtask/tests/corpus_manifest.rs` were written by hand and never formatted, and this toolchain's rustfmt reformats a chained `assert!(expr.method()...)` into a block form. Discriminating check: `cargo fmt --all -- --check`, which named four sites in `corpus.rs` and one import ordering in `corpus_manifest.rs`. Change: `cargo fmt --all`, which rewrote 32 lines across the two files and reordered `use xtask::corpus::{…, MANIFEST_PATH, Manifest}` to the edition 2024 ordering. Result: `cargo fmt --all -- --check` is silent and the lane passes `L0.fmt` at `4d5a48e`. The failing record is committed under `evidence/CHG-002/`, not deleted — CHG-001's repair attempt 4 deleted one and said so; this is the corrected practice.

2. **`L0.typos` failed in CI at `9e7bb89` and had passed locally.** CI run [35478425099](https://github.com/kmosoti/recursive-hexagonal-architecture/actions/runs/35478425099) reported `L0.typos failed: exited with 2`, on `docs/evidence/index.md`: the generated index lists the record filename `20260920T001738Z-038a8ea2ba1c.json`, and typos splits a hex digest on its digits and reads the letter runs as words — here `ba`, "should be `by`, `be`".

   Hypothesis: the local run did not see it because of the order the commands were run in, not because the two machines differ. The lane ran, *then* `cargo xtask docs` regenerated the index to include that very record, and the lane was never re-run. Discriminating check: `typos` on the working tree, which reproduced the CI failure exactly, and `typos --version`, which reports 1.50.2 on both — the same tool, a different input.

   Change: an `extend-ignore-re` of `\b[0-9a-f]{12,}\b` in [`_typos.toml`](../../_typos.toml), which is not a protected surface. Excluding `docs/evidence/index.md` was the alternative and was rejected: digests also appear in change records, task records and acceptance records, so excluding one generated file moves the problem rather than solving it, and it would stop spell-checking that file's prose. Twelve is the shortest hex run this repository uses, the git short revision in a record filename, and no English word is twelve hex characters long.

   Result: `typos` exits 0 on the tree. A probe file inside the repository containing `recieve`, a 12-character digest and a 40-character revision still reports `recieve` and ignores both digests, so the rule masks the false positive and nothing else. The first probe was written to a scratch directory outside the repository, where `_typos.toml` does not apply, and proved nothing; it was re-run inside the tree.

   **Process finding, not a tool finding.** A lane result is only about the tree that existed when it ran. `cargo xtask docs` changes tracked files, so running it after the lane invalidates the record the lane just wrote. CONTRIBUTING tells contributors to regenerate docs after changing `.rha/**` or `evidence/**`; it does not say to re-run the lane afterwards. Queued for the v0.11 proposals as a workflow correction.

## Acceptance concerns

1. **DP-1.1 is the point of this item.** Kennedy reviews the manifest, and authors two or three held-out crate-level cases that live outside this repository. The Executor must not read or write them (§9.14). Every case here was written by the Executor from the plan; the held-out set is the only part of H4 that is not self-assessment, and without it W4's numbers measure the checker against expectations the same party wrote.
2. **The three `[grading]` values are a proposal.** They are the strict reading in each case. If any is relaxed, the relaxation belongs here and in the manifest before CHG-004 runs, not after a result is known.
3. **Still not Eligible.** `L0.architecture` is `not_run` and required and non-waivable under DP-0.5, unchanged since CHG-000's acceptance.
4. **A registered case is not a working fixture.** Nothing has generated a workspace from this manifest yet. The declaration vocabulary is validated for internal consistency only; CHG-004 is where it meets `cargo metadata`, and a case that cannot be expressed is re-registered with its reason *before* that run, per the task record's `replan_when`.
5. **C05 may never reach the checker.** Cargo rejects a package cycle while loading the workspace, so the fixture may fail before `cargo xtask architecture` sees it. The witness carries an `alternative` saying the harness records that error as `graph.cycle`. Whether this counts as the checker detecting the cycle, or as Cargo doing it, is a question for W4.
6. **`cell = "classification"` is not a §4.1 row.** The manifest says so in a comment. If Kennedy would rather §4.1 gained a classification row, that is a v0.11 proposal, not an edit here.
