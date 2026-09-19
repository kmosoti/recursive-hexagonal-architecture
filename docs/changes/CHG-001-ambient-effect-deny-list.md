# CHG-001: Ambient-effect deny list and Clippy configuration discovery

Task record: [`.rha/tasks/CHG-001-ambient-effect-deny-list.toml`](../../.rha/tasks/CHG-001-ambient-effect-deny-list.toml). Plan: [[plan/IMPLEMENTATION-PLAN]] §8 W1. Decision record: [[adr/ADR-0002-clippy-config-discovery]].

The item has three parts. **W1** is the work item the plan asks for. **CHG-001.1** and **CHG-001.2** are rework Kennedy asked for on the open pull request, before merging; their decisions are in the task record.

## Intent and scope

**Behaviour.** A core crate whose `clippy.toml` is the template fails `cargo clippy` on a direct use of any of 80 ambient-effect paths, starting with the seeded `std::time::SystemTime::now()`. The pinned Clippy's configuration discovery and merge behaviour is observed and recorded, together with what switches the deny list off. The rule `effect.core_clippy_template` reports a core crate whose Clippy configuration is not the template.

**Affected components and contracts.** None: no product component exists. New: `xtask/templates/core-clippy.toml` (creation approved by Kennedy, extension approved for CHG-001.2), `xtask/src/clippy_template.rs`, and the Clippy corpus, whose fixture workspaces are generated at test time under `target/clippy-corpus/`.

**Non-goals.** Product crates (CHG-005). Running the rule in L0, since `cargo xtask architecture` is still a stub (CHG-003). The `no_std` option (experiment E1). Any change to the policy, `rha-crates.toml`, `deny.toml`, `rha-baseline.json`, the root `clippy.toml`, the root `Cargo.toml`, or `xtask ci`.

**Mode and responsibilities.** Mode `agent`. Executor `agent:executor`, a role, not a model: the task record's provenance names the model exposed in each span of the work. Planner, Integrator, and Acceptor `human:kennedy`. Authority profile `ECC-Solo`.

## Deltas

**Architecture, API, schema, dependencies, effects, ledger.** A new public xtask module, `clippy_template`, with `check`, `CoreCrate`, `Finding`, and `Problem`. No new dependency. The ledger is unchanged.

**Unsafe, concurrency, authority, policy.** No unsafe code. The only protected surface touched is `xtask/templates/core-clippy.toml`: created in W1 and extended in CHG-001.2, both approved by Kennedy in the conversation and recorded in the task record. `git diff c554316 HEAD` over every other protected surface is empty.

**CHG-001.1, requested rework.**

- `.claude/settings.json` is ignored, so local records no longer report the working tree as dirty because of it.
- The corpus held two committed fixture workspaces: 8 Clippy configuration files, 2 copies of the root lint table, 2 lock files, and a shell script, kept true by drift tests. Each test now generates its fixture under `target/clippy-corpus/` from the three files that own the facts: the root `Cargo.toml`, the root `clippy.toml`, and the template. Eight committed crate sources remain. Two of the three drift tests go with the copies; `template_repeats_every_root_setting` stays, because the template's repeat of the root settings is a copy that rule 2 of ADR-0002 forces.
- `run-experiments.sh` goes. `RHA_CLIPPY_RECORD=<dir>` makes the same test runs write the evidence records, so the experiments a reviewer reads and the experiments L0 runs are one definition.
- The Executor is named by role, `agent:executor`. The plan, the threat model, and the `--principal` help text follow. CHG-000's merged records keep the id they were written with.

**CHG-001.2, deny list and escape hatches.**

- Experiment 6, thirteen runs, records what switches the deny list off and what closes it: `#[allow]`, `#[expect]`, `#![allow]`, and an allow flag in `RUSTFLAGS` all silence it; a crate-level `#![forbid]` or `forbid` in the lint table turns the attributes into `E0453`; `--cap-lints=warn` lowers even a forbidden finding. Experiment 6n shows the same flag works from `.cargo/config.toml`, which is not a protected surface.
- The template grows from 15 entries to 80, closing the gaps ADR-0002 named. The additions come from a sweep of the Rust 1.98.1 standard-library source, limited to the effects §6.8 names. ADR-0002 carries the per-category table and what was deliberately left out.
- `std::env::set_var` and `std::env::remove_var` are `unsafe` in edition 2024, so the fixture names them instead of calling them. The lint fires on the path reference, which is how both entries are demonstrated.

**Removed, weakened, or reinterpreted tests.** The two fixture drift tests are removed with the copies they guarded: `fixtures_copy_the_root_lint_tables_and_root_clippy_file` had nothing left to compare, and the rule's `copies_of_the_template_conform` now writes its own crate directories. No behavioural check was weakened; the corpus went from 4 tests to 7 and from 7 experiments to 21.

**Beyond the plan's three experiments,** each added to make one of them decisive or to answer a listed unknown: 3c (control), 3b (the fix for 3), 4 (every entry), 5 (the plan's unknown, `clippy.toml` beside `.clippy.toml`), and 6a to 6n (what a crate or its configuration can do to the deny list).

## Evidence

**Experiments.** Twenty-one runs, recorded in [`evidence/w1-clippy/`](../../evidence/w1-clippy/) with their literal commands, the digests of every generated file, exit statuses, and captured output. ADR-0002 tabulates them and states what they mean. The first seven records, at commit `28ae543c297067ad5171654f490307b13995376a`, were produced by the shell script that CHG-001.1 replaced; the run in [`evidence/w1-clippy/20260919T221927Z-edd4bac756f9/`](../../evidence/w1-clippy/20260919T221927Z-edd4bac756f9/) is the whole set from the generated corpus. `xtask/tests/clippy_corpus.rs` reruns all of them in every `L0.nextest` run, locally and on the GitHub runner.

**L0 record.** [`evidence/CHG-001/20260919T222005Z-bcaf3090691d.json`](../../evidence/CHG-001/20260919T222005Z-bcaf3090691d.json), class `local`, principal `agent:executor`. Base `c554316` (main); the policy digest equals the base policy's, `sha256:f102c03a…`, so **Applicable holds**.

| Check | Outcome | Detail |
| --- | --- | --- |
| `L0.fmt` | passed | |
| `L0.clippy` | passed | |
| `L0.nextest` | passed | 31 tests, including the 7 corpus tests and 4 rule tests |
| `L0.doctest` | passed | 1 doctest |
| `L0.architecture` | **not_run** | `not_implemented` (stub, exit 4) |
| `L0.deny` | passed | |
| `L0.machete` | passed | |
| `L0.typos` | passed | the corpus sources are excluded by `_typos.toml` |

Eligibility `blocked`: Authentic (no trusted producers) and Passed (`L0.architecture` not_run). Applicable and Complete hold.

**CI records.** Every push to the branch ran the lane on the GitHub runner, image `ubuntu24`, class `ci`. Run [35470532056](https://github.com/kmosoti/recursive-hexagonal-architecture/actions/runs/35470532056) is copied as [`evidence/ci/35470532056.json`](../../evidence/ci/35470532056.json); run [35473050927](https://github.com/kmosoti/recursive-hexagonal-architecture/actions/runs/35473050927), on the merge commit `f7484bd150465966c8fd1ed3eb43966aa9239bdc` of PR head `853e19898a71a7619da6ec1169c769aac5084650` whose tree it shares, is copied as [`evidence/ci/35473050927.json`](../../evidence/ci/35473050927.json). Their outcomes match the local record, and the corpus tests pass in both environments, so the discovery rule and the escape-hatch results reproduce on a second machine.

**Performance.** No trigger; nothing claimed.

### Repair attempts (§11.7.8)

1. **Experiment 3b was not decisive.** In the first scripted run, at `a152d94`, 3b ran against a crate that also held the seeded clock call; that build failed, so the absent `unwrap_used` was weak evidence. Hypothesis: a crate with the template and nothing on the deny list would give a clean exit. Change: a separate fixture crate. Result: exit 0 and no `unwrap_used` (commit `28ae543`).
2. **Experiment 5 exposed a silent override.** Observation, not a repair: with both files present, `.clippy.toml` wins with only a warning. Change: `effect.core_clippy_template` also reports `Problem::Shadowed`. CHG-001.1 made the experiment decisive by putting the deny list in `clippy.toml`, the root settings in `.clippy.toml`, and the seeded call in the crate: the call passes.
3. **Helper `unwrap()` warnings in `xtask/tests/clippy_corpus.rs`.** The root test allowances cover `#[test]` functions and `#[cfg(test)]` modules, not free helper functions in an integration test file. The helpers panic with a message that names the file or fixture.

## Acceptance concerns

1. **Still not Eligible.** `L0.architecture` is `not_run` (not implemented until CHG-003), and it is required and non-waivable under DP-0.5. Kennedy decided on 2026-09-19 to leave it `not_run` rather than move a minimal checker into W1: it would pass over zero core crates, it would not change eligibility while Authentic fails, and it would put checker code before the corpus pre-registration of W2. The options in CHG-000's acceptance concern 2 are unchanged, and the policy was not edited.
2. **No acceptance record for CHG-000.** `.rha/acceptances/` holds only `.gitkeep` on `main`; the acceptance is stated in the message of tag `v0.10-m0`. That directory is yours to write, so nothing was added.
3. **The attribute escape is open.** Rule 5 of ADR-0002: any `#[allow]`, `#[expect]`, or `#![allow]` in a core crate switches the deny list off, and the rule cannot see attributes. Experiment 6 shows two ways to close it, and both are yours: `disallowed_methods`, `disallowed_types`, and `disallowed_macros` set to `forbid` in the root `Cargo.toml` (a protected surface, and it changes nothing for adapters or for clean core crates), or a `#![forbid(...)]` line in each core crate, which the rule would then have to check.
4. **`.cargo/config.toml` is not a protected surface.** Experiment 6n: `--cap-lints=warn` there turns every lint, including a forbidden one, into a warning. The same file already carries the `xtask` alias. Adding it to `[surface] protected` in the policy is your decision.
5. **What the deny list cannot reach.** Direct uses only: not a call through another crate, not one the compiler generates from a macro. `HashMap::new` seeds itself from `RandomState` without naming it, so iteration-order nondeterminism stays out of reach. Platform extension traits and unstable APIs are left out on purpose (ADR-0002).
6. **The rule is not yet in L0.** `effect.core_clippy_template` is exercised only by its unit tests until CHG-003 calls it from `cargo xtask architecture`.
7. **Held-out cases.** None exist for this change. The fixtures and the expected rule were written by the Executor; the expectation was written into the task record before the experiments ran. The deny-list additions were proposed by advisory subagents reading the standard-library source, and every one of them is checked by experiment 4, which fails if an entry stops firing.
