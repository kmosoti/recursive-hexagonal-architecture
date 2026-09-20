# CHG-001: Ambient-effect deny list and Clippy configuration discovery

Task record: [`.rha/tasks/CHG-001-ambient-effect-deny-list.toml`](../../.rha/tasks/CHG-001-ambient-effect-deny-list.toml). Plan: [[plan/IMPLEMENTATION-PLAN]] §8 W1. Decision record: [[adr/ADR-0002-clippy-config-discovery]].

The item has four parts. **W1** is the work item the plan asks for. **CHG-001.1**, **CHG-001.2** and **CHG-001.3** are rework Kennedy asked for on the open pull request, before merging; their decisions are in the task record.

## Intent and scope

**Behaviour.** A core crate whose `clippy.toml` is the template fails `cargo clippy` on a direct use of any of 111 ambient-effect paths, starting with the seeded `std::time::SystemTime::now()`. The pinned Clippy's configuration discovery and merge behaviour is observed and recorded, together with what switches the deny list off. The rule `effect.core_clippy_template` reports a core crate whose Clippy configuration is not the template.

**Affected components and contracts.** None: no product component exists. New: `xtask/templates/core-clippy.toml` (creation approved by Kennedy, extension approved for CHG-001.2), `xtask/src/clippy_template.rs`, and the Clippy corpus, whose fixture workspaces are generated at test time under `target/clippy-corpus/`.

**Non-goals.** Product crates (CHG-005). Running the rule in L0, since `cargo xtask architecture` is still a stub (CHG-003). The `no_std` option (experiment E1). Any change to the policy, `rha-crates.toml`, `deny.toml`, `rha-baseline.json`, the root `clippy.toml`, the root `Cargo.toml`, or `xtask ci`.

**Mode and responsibilities.** Mode `agent`. Executor `agent:executor`, a role, not a model: the task record's provenance names the model exposed in each span of the work. Planner, Integrator, and Acceptor `human:kennedy`. Authority profile `ECC-Solo`.

## Deltas

**Architecture, API, schema, dependencies, effects, ledger.** A new public xtask module, `clippy_template`, with `check`, `CoreCrate`, `Finding`, and `Problem`. No new dependency. The ledger is unchanged.

**Unsafe, concurrency, authority, policy.** No unsafe code. The only protected surface touched is `xtask/templates/core-clippy.toml`: created in W1 and extended in CHG-001.2, both approved by Kennedy in the conversation and recorded in the task record. `git diff c554316 HEAD` over every other protected surface is empty.

**CHG-001.1, requested rework.**

- `.claude/settings.json` is ignored, so local records no longer report the working tree as dirty because of it.
- The corpus held two committed fixture workspaces: 6 Clippy configuration files, which made 8 in the repository with the root file and the template, 2 copies of the root lint table, 2 lock files, and a shell script, kept true by drift tests. Seventeen committed fixture files and the script go. Each test now generates its fixture under `target/clippy-corpus/` from the three files that own the facts: the root `Cargo.toml`, the root `clippy.toml`, and the template. Eight committed crate sources remain. Two of the three drift tests go with the copies; `template_repeats_every_root_setting` stays, because the template's repeat of the root settings is a copy that rule 2 of ADR-0002 forces.
- `run-experiments.sh` goes. `RHA_CLIPPY_RECORD=<dir>` makes the same test runs write the evidence records, so the experiments a reviewer reads and the experiments L0 runs are one definition.
- The Executor is named by role, `agent:executor`. The plan, the threat model, and the `--principal` help text follow. CHG-000's merged records keep the id they were written with.

**CHG-001.2, deny list and escape hatches.**

- Experiment 6, fourteen runs, records what switches the deny list off and what closes it: `#[allow]`, `#[expect]`, `#![allow]`, and an allow flag in `RUSTFLAGS` all silence it; a crate-level `#![forbid]` or `forbid` in the lint table turns the attributes into `E0453`; `--cap-lints=warn` lowers even a forbidden finding. Experiment 6n shows the same flag works from `.cargo/config.toml`, which is not a protected surface.
- The template grows from 15 entries to 111, closing the gaps ADR-0002 named. Ten of the additions are the paths that record listed; the rest come from a sweep of the Rust 1.98.1 standard-library source and two adversarial reviews of the result, all limited to the effects §6.8 names. ADR-0002 carries the per-category table, the candidates left for Kennedy, and what cannot be reached at all. Experiment 4 shows every entry resolve and fire.
- `std::env::set_var` and `std::env::remove_var` are `unsafe` in edition 2024, so the fixture names them instead of calling them. The lint fires on the path reference, which is how both entries are demonstrated.

**CHG-001.3, acceptance, the escape, and the front door.** Kennedy asked for the acceptance records, the recommended decisions, the findings in the specification, and a README worth reading.

- [`.rha/acceptances/CHG-000.toml`](../../.rha/acceptances/CHG-000.toml) records the acceptance Kennedy made on 2026-09-19 by merging PR 1 and tagging `v0.10-m0`: the subject, the evidence, the outcomes as recorded, the four predicates, and the six concerns it carries forward rather than closes. No acceptance record is written for CHG-001, which is neither merged nor accepted.
- **The recommended `forbid` was refused by the toolchain.** Set on the three lints in the root `Cargo.toml`, it stopped `xtask` compiling with 15 `E0453` errors, all from `clap`'s derives, which expand to `#[allow(clippy::style)]`. Experiments 7a and 7b reproduce the mechanism without a dependency; a `serde` derive is unaffected. The escape is closed per core crate instead, by a crate-root `#![forbid(...)]` that experiment 7c shows costs clean code nothing, and the rule reports a crate root that leaves any of the three unforbidden.
- `.cargo/config.toml` joins the protected surfaces in `.rha/policy.toml`, because `--cap-lints=warn` there disables every lint at once.
- Spec §6.8 asked the implementer to verify how the pinned Clippy finds and merges configuration files. It is verified. The first edit put the five observations in §6.8 as a table, which duplicated ADR-0002 in a second editable place, against the one-owner rule of §11.7.1 and the defect §12.1 records against v0.7. Corrected on the same day: §6.8 keeps the normative consequence in its Hole column and a sentence saying where the observations live, and the proposals that would change its text are queued in [`docs/proposals/spec-v0.11.md`](../proposals/spec-v0.11.md) for the version bump. That file also states the working rule it came from: a work item records findings in a decision record and evidence, the specification changes in one item per version, and a claim the specification makes about this repository gets a check. The §1.4 maturity table is untouched.
- The README and the GitHub description describe the project, what works today, and what is only specified.

**Removed, weakened, or reinterpreted tests.** The two fixture drift tests are removed with the copies they guarded: `fixtures_copy_the_root_lint_tables_and_root_clippy_file` had nothing left to compare, and the rule's `copies_of_the_template_conform`, renamed `a_copy_of_the_template_conforms`, now writes its own crate directories. No behavioural check was weakened; the corpus went from 4 tests to 7 and from 7 experiments to 21.

**Beyond the plan's three experiments,** each added to make one of them decisive or to answer a listed unknown: 3c (control), 3b (the fix for 3), 4 (every entry), 5 (the plan's unknown, `clippy.toml` beside `.clippy.toml`), and 6a to 6n (what a crate or its configuration can do to the deny list).

## Evidence

**Experiments.** Twenty-six experiments, each recorded with its literal command, the digests of every generated file, the exit status, and the captured output, in [`evidence/w1-clippy/20260919T231239Z-8af79cb0f76a/`](../../evidence/w1-clippy/20260919T231239Z-8af79cb0f76a/), written from the generated corpus at commit `8af79cb0f76ad5ea0386f3a2af85dc9b65dd48b1` with a clean working tree. That commit carries the template this record describes, so each record's digest of `crates/core-every/clippy.toml` equals the template's. Two earlier sets are superseded: one at `edd4bac` against an 80-entry template, and one at `bf5aed9` that predates experiments 7a to 7c. [`evidence/w1-clippy/`](../../evidence/w1-clippy/) also keeps the seven records of the first run, at commit `28ae543c297067ad5171654f490307b13995376a`, which the shell script produced before CHG-001.1 replaced it. ADR-0002 tabulates the experiments and states what they mean. `xtask/tests/clippy_corpus.rs` reruns all of them in every `L0.nextest` run, locally and on the GitHub runner.

**L0 records.** Two matter, and they differ in one predicate. [`evidence/CHG-001/20260919T224504Z-bae4b34894a2.json`](../../evidence/CHG-001/20260919T224504Z-bae4b34894a2.json) covers the deny-list work under the unchanged policy, where the digest equals the base policy's and **Applicable holds**. [`evidence/CHG-001/20260919T231351Z-602d2f953d0b.json`](../../evidence/CHG-001/20260919T231351Z-602d2f953d0b.json) covers the head of this change, after `.cargo/config.toml` joined the protected list, and reports **Applicable false**, naming the two digests. Both are class `local`, principal `agent:executor`, on a clean working tree. The table below is the later one.

| Check | Outcome | Detail |
| --- | --- | --- |
| `L0.fmt` | passed | |
| `L0.clippy` | passed | |
| `L0.nextest` | passed | 33 tests, including the 7 corpus tests and 6 rule tests |
| `L0.doctest` | passed | 1 doctest |
| `L0.architecture` | **not_run** | `not_implemented` (stub, exit 4) |
| `L0.deny` | passed | |
| `L0.machete` | passed | |
| `L0.typos` | passed | the corpus sources are excluded by `_typos.toml` |

Eligibility `blocked`: Authentic (no trusted producers), Applicable (the policy changed in this contribution), and Passed (`L0.architecture` not_run). Complete holds.

**CI records.** Every push to the branch ran the lane on the GitHub runner, image `ubuntu24`, class `ci`. Two runs are committed, and each covers a different revision. Run [35470532056](https://github.com/kmosoti/recursive-hexagonal-architecture/actions/runs/35470532056), copied as [`evidence/ci/35470532056.json`](../../evidence/ci/35470532056.json), covers the W1 head `d011d13` and its 28 tests; it predates the rework, so it says nothing about the escape hatches. Run [35474230710](https://github.com/kmosoti/recursive-hexagonal-architecture/actions/runs/35474230710), copied as [`evidence/ci/35474230710.json`](../../evidence/ci/35474230710.json), covers PR head `1d65ce72e8c52ddcadd9354795fad3c163d56fec`, the deny-list work. Run [35475553364](https://github.com/kmosoti/recursive-hexagonal-architecture/actions/runs/35475553364), copied as [`evidence/ci/35475553364.json`](../../evidence/ci/35475553364.json), covers PR head `df8ec0fc773bcc4f2a106b414540bffc4bd14808`. Run [35476272071](https://github.com/kmosoti/recursive-hexagonal-architecture/actions/runs/35476272071), copied as [`evidence/ci/35476272071.json`](../../evidence/ci/35476272071.json), covers PR head `5c24f54272eb8d90c878bf43ff96daf67c87d551`, the head this record describes, and shares its tree. Its outcomes match the local record, so the discovery rule and the escape-hatch results reproduce on a second machine.

**Performance.** No trigger; nothing claimed.

### Repair attempts (§11.7.8)

1. **Experiment 3b was not decisive.** In the first scripted run, at `a152d94`, 3b ran against a crate that also held the seeded clock call; that build failed, so the absent `unwrap_used` was weak evidence. Hypothesis: a crate with the template and nothing on the deny list would give a clean exit. Change: a separate fixture crate. Result: exit 0 and no `unwrap_used` (commit `28ae543`).
2. **Experiment 5 exposed a silent override.** Observation, not a repair: with both files present, `.clippy.toml` wins with only a warning. Change: `effect.core_clippy_template` also reports `Problem::Shadowed`. CHG-001.1 made the experiment decisive by putting the deny list in `clippy.toml`, the root settings in `.clippy.toml`, and the seeded call in the crate: the call passes.
3. **Helper `unwrap()` warnings in `xtask/tests/clippy_corpus.rs`.** The root test allowances cover `#[test]` functions and `#[cfg(test)]` modules, not free helper functions in an integration test file. The helpers panic with a message that names the file or fixture.
4. **The lane caught the rule code.** At `043d7c1`, `L0.clippy` failed: the new crate-root check ended a closure's mutable borrow with `drop(finding)`, which `clippy::drop_non_drop` rejects because a closure implements no `Drop`. Hypothesis: a block scope would end the borrow without the call. Result: the lane passes at `602d2f9`. The record of the failing run was deleted rather than committed, which was the wrong call in a repository that keeps what it observes; this paragraph stands in its place.

## Acceptance concerns

1. **Still not Eligible.** `L0.architecture` is `not_run` (not implemented until CHG-003), and it is required and non-waivable under DP-0.5. Kennedy decided on 2026-09-19 to leave it `not_run` rather than move a minimal checker into W1: it would pass over zero core crates, it would not change eligibility while Authentic fails, and it would put checker code before the corpus pre-registration of W2. The options in CHG-000's acceptance concern 2 are unchanged, and the policy was not edited.
2. **The policy changed in this contribution, so Applicable no longer holds.** Adding `.cargo/config.toml` to the protected list changes the policy digest, and a record is Applicable only where the candidate's policy equals the base revision's (§11.7.6). The L0 record of CHG-001.2, taken before that edit, is the one that holds under the unchanged policy; the record taken after it reports Applicable false with that reason. §11.5 calls this a controlled transition, and it is yours to accept.
3. **The specification carries two sentences of this change.** §6.8's Hole column states the consequence, and one sentence says where the observations live; everything else is queued for v0.11. The §1.4 maturity table is untouched, because those rows are yours to promote at a version bump from `docs/maturity.md`. Reverting either sentence costs nothing but the text.
4. **The attribute escape is closed by convention plus a rule, not by the compiler.** A core crate that carries the crate-root `#![forbid(...)]` cannot allow these lints anywhere, including its own tests; a core crate that omits the line is reported by `effect.core_clippy_template`, which does not run in L0 until CHG-003. Until then nothing enforces the line, and no core crate exists to carry it.
5. **What the deny list cannot reach.** Direct uses only: not a call through another crate, not one the compiler generates from a macro. `HashMap::new` seeds itself from `RandomState` without naming it, so iteration-order nondeterminism stays out of reach. Platform extension traits and unstable APIs are left out on purpose (ADR-0002).
6. **The rule is not yet in L0.** `effect.core_clippy_template` is exercised only by its unit tests until CHG-003 calls it from `cargo xtask architecture`.
7. **Held-out cases.** None exist for this change. The fixtures and the expected rule were written by the Executor; the expectation was written into the task record before the experiments ran. The deny-list additions were proposed by advisory subagents reading the standard-library source, and every one of them is checked by experiment 4, which fails if an entry stops firing.

## Acceptance (CHG-001.4)

Kennedy accepted this change at `44b1e44`, the merge commit of [pull request 2](https://github.com/kmosoti/recursive-hexagonal-architecture/pull/2), on 2026-09-19. The record is [`.rha/acceptances/CHG-001.toml`](../../.rha/acceptances/CHG-001.toml), written by the Executor at his instruction, which is also the approval for that protected surface.

The merge added nothing of its own: `44b1e44^{tree}` equals `a31cdd1^{tree}`, both `336f32c8`, and `git diff a31cdd1 44b1e44` is empty.

**The evidence that covers the accepted tree.** The records committed before the merge stop short of it. The last local record, [`evidence/CHG-001/20260919T232940Z-78fe0093c4a4.json`](../../evidence/CHG-001/20260919T232940Z-78fe0093c4a4.json), was taken at `78fe009`, two commits before the head; run 35476272071 covers head `5c24f54`. CHG-001.4 therefore downloads the `rha-evidence` artifact of run [35476379661](https://github.com/kmosoti/recursive-hexagonal-architecture/actions/runs/35476379661), the run on head `a31cdd1`, and commits it as [`evidence/ci/35476379661.json`](../../evidence/ci/35476379661.json). Its `artifact_identity.tree` is `336f32c8968d8f9b523cae4d96f168ba59c485c9`, equal to the accepted tree, so the lane ran on exactly the content now on `main`; its `revision` is GitHub's ephemeral merge commit `5343431`, which no longer exists as a branch. Seven checks passed, `L0.architecture` is `not_run`, and all four predicates match the local record.

That the record arrives after the item it describes is itself a weakness, listed in the acceptance record's `carried_forward`: an acceptance whose evidence has to be fetched from a CI artifact is reproducible only while the artifact lives.

**The transition closed.** Every cited record reports `base_policy_digest` `sha256:f102c03a…` against its own `policy_digest` `sha256:dc2ef7e0…`, so Applicable is false on all of them. A lane run at `44b1e44` reports both as `sha256:dc2ef7e0…` and Applicable true: the base is now the policy this change proposed. That run is reported in the working conversation and is not committed, so `main` carries no record of its own.

### Defect found at acceptance

**D-001.1, record integrity.** The task record shipped at `44b1e44` with the literal string `SPEC_DIGEST` where the sha256 of `docs/spec/rha-spec-v0.10.md` belonged, in the fifth `instruction_sources` entry. The record asserted a witness it did not hold, in the one field that lets a reader reproduce the Executor's inputs. Nothing caught it: no check reads task records, and the record-schema check that would reject a non-hex digest belongs to CHG-019. CHG-001.4 fills in `bd96216829035ade64c43befa5e672c58078f6afa4b04145c6183227e4e1071a`, verifiable with `git show 44b1e44:docs/spec/rha-spec-v0.10.md | sha256sum`, and the acceptance record states the defect in full.
