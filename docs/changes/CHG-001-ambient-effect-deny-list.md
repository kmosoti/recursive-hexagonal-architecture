# CHG-001: Ambient-effect deny list and Clippy configuration discovery

Task record: [`.rha/tasks/CHG-001-ambient-effect-deny-list.toml`](../../.rha/tasks/CHG-001-ambient-effect-deny-list.toml). Plan: [[plan/IMPLEMENTATION-PLAN]] §8 W1. Decision record: [[adr/ADR-0002-clippy-config-discovery]].

## Intent and scope

**Behaviour.** A core crate whose `clippy.toml` is the template fails `cargo clippy` on a direct call to any of 15 ambient-effect paths, starting with the seeded `std::time::SystemTime::now()`. The pinned Clippy's configuration discovery and merge behaviour is observed and recorded. The rule `effect.core_clippy_template` reports a core crate whose Clippy configuration is not the template.

**Affected components and contracts.** None: no product component exists. New: `xtask/templates/core-clippy.toml` (creation approved by Kennedy), `xtask/src/clippy_template.rs`, and two corpus workspaces under `xtask/tests/corpus/clippy/`, both excluded from the root workspace.

**Non-goals.** Product crates (CHG-005). Running the rule in L0, since `cargo xtask architecture` is still a stub (CHG-003). The `no_std` option (experiment E1). Any change to the policy, `rha-crates.toml`, `deny.toml`, `rha-baseline.json`, the root `clippy.toml`, or `xtask ci`.

**Mode and responsibilities.** Mode `agent`. Executor `agent:claude-opus-5`. Planner, Integrator, and Acceptor `human:kennedy`. Authority profile `ECC-Solo`.

## Deltas

**Architecture, API, schema, dependencies, effects, ledger.** A new public xtask module, `clippy_template`, with `check`, `CoreCrate`, `Finding`, and `Problem`. No new dependency. The ledger is unchanged.

**Unsafe, concurrency, authority, policy.** No unsafe code. The only protected surface touched is the creation of `xtask/templates/core-clippy.toml`, which Kennedy pre-approved; `git diff c554316 HEAD` over the other protected surfaces is empty. `.gitignore` now ignores the fixture workspaces' `target/` directories.

**Removed, weakened, or reinterpreted tests.** None.

**Beyond the plan's three experiments,** each added to make one of them decisive or to answer a listed unknown:

- **3c, control for 3:** the same test `unwrap()` in the adapter, whose nearest file is the fixture root. It shows the root allowance works where that file is the nearest, so experiment 3's `unwrap_used` comes from replacement, not from a broken allowance.
- **3b, the fix for 3:** a core crate with the template, which repeats the allowances, and nothing on the deny list.
- **4, every entry:** one use of each of the 15 template entries, so a misspelled or unresolved path would show up as a missing finding.
- **5, the plan's unknown:** `clippy.toml` and `.clippy.toml` in one directory.

## Evidence

**Experiments.** Seven records in [`evidence/w1-clippy/`](../../evidence/w1-clippy/), produced by `xtask/tests/corpus/clippy/run-experiments.sh evidence/w1-clippy` at commit `28ae543c297067ad5171654f490307b13995376a`. The working tree was dirty only through untracked files that feed no check: Kennedy's `.claude/settings.json` and the records being written.

| # | Command (in fixture) | Exit | Decisive stderr |
| --- | --- | --- | --- |
| 1 | `cargo clippy -p core-a` (core-seeded) | 101 | ``error: use of a disallowed method `std::time::SystemTime::now` ``; `` = note: `-D clippy::disallowed-methods` implied by `-D clippy::all` `` |
| 2 | `cargo clippy -p adapter-x` (discovery) | 0 | none: no deny-list finding |
| 3 | `cargo clippy -p core-a --all-targets` (discovery) | 0 | ``warning: used `unwrap()` on a `Result` value``: `unwrap_used` **fired** |
| 3c | `cargo clippy -p adapter-x --all-targets` (discovery) | 0 | none: no `unwrap_used` |
| 3b | `cargo clippy -p core-b --all-targets` (core-seeded) | 0 | none: no `unwrap_used` |
| 4 | `cargo clippy -p core-every` (core-seeded) | 101 | all 15 entries: `use of a disallowed method/type/macro …`, 18 errors in total |
| 5 | `cargo clippy` (scratch pair) | 0 | ``warning: using config file `…/.clippy.toml`, `…/clippy.toml` will be ignored`` |

The observed rule matches the plan's expectation: per-crate discovery, no merge, and the dotfile wins. The `CLIPPY_CONF_DIR` fallback is not used. `xtask/tests/clippy_corpus.rs` reruns experiments 1, 2, 3, 3c, 3b, and 4 on every `L0.nextest` run.

**L0 record.** Pending: filled in after the run on the committed candidate.

**CI record.** Pending: filled in after the first CI run on the pull request.

**Performance.** No trigger; nothing claimed.

### Repair attempts (§11.7.8)

1. **Experiment 3b was not decisive.** In the first scripted run, at `a152d94`, 3b ran `cargo clippy -p core-a --all-targets` in `core-seeded`, where `core-a` also holds the seeded clock call; that build failed, so the absent `unwrap_used` was weak evidence. Hypothesis: a crate with the template and nothing on the deny list would give a clean exit. Change: a new fixture crate `core-b`, with 3b pointed at it. Result: exit 0 and no `unwrap_used` (commit `28ae543`).
2. **Experiment 5 exposed a silent override.** Observation, not a repair: with both files present, `.clippy.toml` wins with only a warning. Change: `effect.core_clippy_template` also reports `Problem::Shadowed` for a `.clippy.toml` in a core crate's directory, with a unit test.
3. **Helper `unwrap()` warnings in `xtask/tests/clippy_corpus.rs`.** The root test allowances cover `#[test]` functions and `#[cfg(test)]` modules, not free helper functions in an integration test file. The helpers now panic with a message that names the file or fixture.

## Acceptance concerns

1. **Still not Eligible.** `L0.architecture` is `not_run` (not implemented until CHG-003), and it is required and non-waivable under DP-0.5. The options in CHG-000's acceptance concern 2 are unchanged, and the policy was not edited.
2. **No acceptance record for CHG-000.** `.rha/acceptances/` holds only `.gitkeep` on `main`; the acceptance is stated in the message of tag `v0.10-m0`. That directory is yours to write, so nothing was added.
3. **Untracked `.claude/settings.json`.** It makes every local record in this change `dirty: true` and lists it as an untracked input; it feeds no check. Committing it or ignoring it is your choice.
4. **Deny-list gaps.** ADR-0002 lists known effectful paths the template does not cover, such as `SystemTime::elapsed`, `OpenOptions`, `std::env::args`, and `std::process::exit`. Extending the template is a protected change and your decision.
5. **The rule is not yet in L0.** `effect.core_clippy_template` is exercised only by its unit tests until CHG-003 calls it from `cargo xtask architecture`.
6. **Held-out cases.** None exist for this change. The fixtures and the expected rule were written by the Executor; the expectation was written into the task record before the experiments ran.
