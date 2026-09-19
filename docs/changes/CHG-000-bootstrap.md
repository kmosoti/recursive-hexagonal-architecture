# CHG-000: Bootstrap

Task record: [`.rha/tasks/CHG-000-bootstrap.toml`](../../.rha/tasks/CHG-000-bootstrap.toml). Plan: [[plan/IMPLEMENTATION-PLAN]] §8 W0, layout §4.1. Decisions: [[adr/ADR-0001-bootstrap-decisions]].

## Intent and scope

**Behaviour.** The repository runs the spec's §12.1 fast lane through one entry point, `cargo xtask ci`, and records the outcome of each check in an evidence record with the five states of §11.4. CI runs the same entry point on every pull request and push. The contribution artifacts of §11.7.3 to §11.7.5 exist: AGENTS.md, CONTRIBUTING.md, the PR template, task and change records.

**Affected components and contracts.** None: no product component exists. The only workspace member is `xtask` (role `tool`).

**Non-goals.** Product crates (CHG-005); crate-graph rules, since `cargo xtask architecture` is a stub reporting `not_run` (CHG-003); the H4 corpus (CHG-002, CHG-004); the L1 evaluator `cargo xtask l1`; record schemas (CHG-019).

**Mode and responsibilities.** Mode `agent`. Executor `agent:claude-opus-5`. Planner, Integrator, and Acceptor `human:kennedy`. Authority profile `ECC-Solo`. Verifiers: `xtask ci` locally and GitHub Actions, both advisory.

## Deltas

**Architecture, API, schema, dependencies, effects, ledger.** New repository. One workspace member, `xtask` 0.1.0, with dependencies `clap` 4.6.7, `serde` 1.0.229, `serde_json` 1.0.151, `sha2` 0.11.0, and `toml` 1.1.6. The plan's registry cache listed `sha2` 0.10.x; `cargo search` showed 0.11.0 as current, so 0.11.0 is pinned. `cargo deny` passes with licences MIT, Apache-2.0, and Unicode-3.0. The assumption ledger and the architecture declarations are empty.

**Unsafe, concurrency, authority, policy.** No unsafe code. This change creates the first machine policy, `.rha/policy.toml`, with parameters from DP-0.5, and the branch protection of DP-0.6. It changes authority by existing: no earlier policy governs it.

**Removed, weakened, or reinterpreted tests.** None; there were no tests before.

**Deviations from the plan**, each also in ADR-0001 or the task record:

1. Workspace `members` is `["crates/*", "xtask"]`, as spec §6.4 prints it, not the plan's list with `"tools/*"`. Cargo reads a member glob that matches nothing as a literal path and fails (observed). For the same reason `crates/.gitkeep` is load-bearing until the first crate exists (observed: removing it breaks `cargo metadata`).
2. Generated docs name their sources with a sha256 digest, not a git revision; a revision would make `cargo xtask docs --check` stale after every commit.
3. `cargo xtask ci` exits 1 only when a check failed or, under `--label ci`, a tool is missing. A required check that is `not_run` does not change the exit status, which is why the `ci` status can be green while eligibility is `blocked`; see acceptance concern 3.
4. Git identity uses the GitHub noreply address; see DP-0.1 in the task record.
5. cargo-mutants and cargo-modules are not installed; that decision stays at DP-2.1, as the plan allows.
6. The evidence record extends the plan's §7 shape: `lane`, `started_at` and `finished_at`, `artifact_identity.snapshot_tree` and `git_object_format`, `verification_identity.policy_read_from`, `base_revision`, and `base_policy_digest`, a per-check `tool.probe`, `required`, and `waivable`, record-level `limits`, and `disposition.predicates`, which reports Authentic, Applicable, Complete, and Passed separately.
7. `_typos.toml` accepts the word "Mor". It is a given name in the spec's reference [R122] (Mor Harchol-Balter), which typos flagged as a misspelling of "More"; the spec is not the Executor's to edit.
8. The L1 check `L1.docs_fresh` is declared in the policy, but nothing evaluates L1 triggers yet, so no L1 check runs in CI.

## Evidence

**Candidate.** Commit `c48f1bd1f44dd7dce90f78de78e41f22bfd7ff54`, tree `53a38cc1efd282a70f53b91f982706f6f0f94baa`, clean; `snapshot_tree` equals `tree`. Base `81641865a4fe2a9ae6c0b660e03a8000456d6ff3`, the spec-only first commit.

**Policy.** `.rha/policy.toml`, `sha256:f102c03ad7d2ec0f994ebfd2ead0b1484eb8b03b4dca6f32e266596525c49d86`, read from the candidate. The base has no policy (`base_policy_digest: absent`).

**Local record.** [`evidence/CHG-000/20260919T205530Z-c48f1bd1f44d.json`](../../evidence/CHG-000/20260919T205530Z-c48f1bd1f44d.json), class `local`, principal `agent:claude-opus-5`.

| Check | Outcome | Detail |
| --- | --- | --- |
| `L0.fmt` | passed | |
| `L0.clippy` | passed | no warnings |
| `L0.nextest` | passed | 20 tests selected, 0 failures |
| `L0.doctest` | passed | 1 doctest |
| `L0.architecture` | **not_run** | `not_implemented` (stub, exit 4, report outcome `not_run`) |
| `L0.deny` | passed | advisories, bans, licences, sources |
| `L0.machete` | passed | |
| `L0.typos` | passed | |

Eligibility `blocked`. Authentic: no trusted producers. Applicable: base has no policy. Passed: `L0.architecture` is `not_run`. Complete holds.

**CI record.** Pending: filled in after the first CI run on the pull request.

**Other checks run for this item.**

- `cargo xtask ci --print` printed the eight §12.1 commands in order.
- Drift detection: changing `L0.fmt`'s argv to `cargo fmt --all` made `cargo test -p xtask --test policy_drift` fail with ``command 1: policy `cargo fmt --all` != spec `cargo fmt --all -- --check` ``. Restoring the file (sha256 identical to before) made it pass.
- Branch protection, read back with `gh api`: required status `ci`, `strict: true`, `enforce_admins: true`, 0 required approvals, force-push and deletion disallowed.
- The four `install-action` tool versions exist in that action's manifests at the pinned commit `9114bf4`.
- The workflow's `jq` tool-list expression, run locally on `rha-baseline.json`, printed `cargo-nextest@0.9.145,cargo-deny@0.20.2,cargo-machete@0.9.2,typos@1.50.2`.

**Performance.** No performance trigger; nothing claimed.

### Repair attempts (§11.7.8)

1. **`tools/*` member glob.** Hypothesis: Cargo treats a member glob with no matches as a literal path. Discriminating check: `cargo metadata` with and without `crates/.gitkeep`; it fails exactly when `crates/*` matches nothing. Change: `members` as §6.4 prints it. Result: builds.
2. **cargo-machete under `cargo xtask`.** The dry run recorded `L0.machete` as `not_run` because `cargo machete --version` exited 2 with "Analyzing dependencies of crates in machete,--version". Hypothesis: `cargo run` puts `CARGO_PKG_NAME` in xtask's environment, and cargo-machete uses that variable to decide it was not started by cargo. Discriminating check in a plain shell: `cargo machete --version` printed `0.9.2`, while `CARGO_PKG_NAME=xtask cargo machete --version` reproduced the failure, and `CARGO_PKG_NAME=xtask cargo machete` exited 2 on the nonexistent path `machete`. Change: `util::command` removes the `CARGO_PKG_*` and per-package variables from every child process. Result: `L0.machete` passed.
3. **Push rejected.** GitHub refused the first push (GH007, private email). Not a repair of this change: DP-0.1 was asked again, and Kennedy chose the noreply address.

## Acceptance concerns

1. **Bootstrap acceptance.** Base `8164186` has no policy, so no approved policy governs CHG-000, and Applicable fails by construction. No record is Authentic until CHG-020. Accepting CHG-000 is Kennedy's decision on a controlled transition (§11.5), not `MergeAllowed` of §11.7.6.
2. **`L0.architecture` blocks CHG-001 and CHG-002 under this policy.** DP-0.5 makes every L0 check required and non-waivable, and `L0.architecture` cannot pass before CHG-003. The plan orders W1 and W2 before W3, and W2 must precede W3, because the corpus is pre-registered before any checker code exists. Options:
   - (a) Make `L0.architecture` waivable until CHG-003 is accepted, and issue a logged ECC-Solo exception, with the 24 h cooling-off, for CHG-001 and CHG-002. Recommended: it is the spec's exception path, it is visible in `.rha/exceptions.log`, and it expires.
   - (b) Treat CHG-001 and CHG-002 as controlled transitions like CHG-000.
   - (c) Leave the policy unchanged and accept that those two items stay ineligible.

   Any change to the policy is a protected-surface edit and needs your approval.
3. **Meaning of the green `ci` status.** It means no check failed and no tool was missing. It does not mean eligible; eligibility is only in the record. Review this choice (ADR-0001, decision 9).
4. **CI runs candidate code.** The workflow executes the candidate's own `xtask` and policy, so CI records are class `ci`, not protected; see [[threat-model]].
5. **No licence file.** The repository is public, and without a LICENSE it is all rights reserved by default. This is your decision; nothing was added.
6. **Harness instruction discovery (§11.7.2).** Claude Code loads `CLAUDE.md`, not `AGENTS.md`, so AGENTS.md's effect on this Executor is unverified. A later item could add a `CLAUDE.md` that imports `@AGENTS.md`, with the §11.7.2 discovery tests. That is out of scope here.
7. **AGENTS.md was adopted without the ablation** that §11.7.3 and H6 ask for; H6 is deferred (plan §10).
8. **No held-out acceptance check exists for this change.** Your review of the policy and the verifier code is the substitute.
9. **Milestone M0.** After merging, tagging `v0.10-m0` is yours to do; the Executor does not tag.
