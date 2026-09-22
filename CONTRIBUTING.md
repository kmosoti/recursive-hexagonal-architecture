# Contributing

The shared workflow for human and agent contributors (spec §11.7.4). The machine policy, [`.rha/policy.toml`](.rha/policy.toml), owns check ids and parameters; this guide links to it instead of copying it.

## Start here

- **Toolchain.** `rust-toolchain.toml` pins Rust 1.98.1 with rustfmt, clippy, and rust-src. Run `rustup toolchain install` in the repository root.
- **Tools.** [docs/toolchain.md](docs/toolchain.md) lists every tool, its required version, and its install command; it is generated from `rha-baseline.json`. A tool that is absent or at another version makes its check `not_run`, never `passed`.
- **The verification command.**

  ```sh
  cargo xtask ci                     # run the L0 lane; writes target/rha/evidence.json
  cargo xtask ci --print             # list the lane's commands, in order
  cargo xtask ci --record evidence/CHG-0nn --principal <you>   # keep the record with the change
  ```

  Lane membership is owned by spec [§12.1](docs/spec/rha-spec-v0.10.md#121-lanes-the-single-owner-of-check-membership-and-cadence); the policy mirrors it, and `xtask/tests/policy_drift.rs` fails when the two differ.
- **What exists at this revision.**

  | Mechanism | Status |
  | --- | --- |
  | `cargo xtask ci` (a lane from the policy, evidence record) | implemented (CHG-000) |
  | `cargo xtask docs [--check]` (generated docs) | implemented (CHG-000) |
  | `cargo xtask architecture` | implemented at crate level (CHG-003.1); H4 corpus validation pending (CHG-004); module checks (CHG-007) and transitive analysis unavailable |
  | `cargo xtask l1` (risk-triggered lane) | not implemented; L1 checks do not run |
  | Record schemas, `cargo xtask rha …` | not implemented (CHG-019 onward) |
  | Protected verifier | not implemented (CHG-020); every record is advisory |

## Define the change

Each change is one work item with a task record, `.rha/tasks/CHG-0nn-<slug>.toml`, written before the code. It states the observable behaviour, the affected components and contracts, non-goals, compatibility and resource implications, the contribution mode, and the actors and roles (spec §11.6). Copy the shape of an existing record. A change that creates a crate needs a boundary decision record in `docs/adr/` whose refutation criterion is written first (spec §7.8 step 10, §7.9).

Before authoring, derive assertions from the owned contract for invalid, missing,
unsupported, and non-default input. A negative regression must fail before the
repair and its nearest valid control must pass. After repairing a producer,
validation rule, or identity field, trace the complete pipeline through the
consumer, exit status, and report.

## Implement

- One coherent patch per work item, on a branch `chg/0nn-<slug>`, and one PR per item.
- Justify each new dependency, crate, port, or effect (spec §11.1).
- **Tests are not weakened silently** (spec §9.14). A change that removes, skips, weakens, or reinterprets a test says in its change record whether it corrects the test, changes an approved requirement, or changes the assurance strategy. A green result is not a justification.
- The protected surfaces listed in `.rha/policy.toml` under `[surface]` change only with explicit approval from the acceptance authority, stated in the PR.
- Each repair attempt records its hypothesis, discriminating check, change, and result in the change record. After `repair_loop.max_attempts` unexplained attempts, stop and escalate.

## Verify

- Run `cargo xtask ci`. Every check ends in one of five states (spec §11.4): `passed`, `failed`, `not_run` with a reason, `not_applicable` (root policy only), or `inconclusive` (comparisons only). A check that did not run is `not_run`; never report it as passed.
- `passed` needs exit status 0 **and** the check's validity criterion; for example, nextest must select at least one test.
- The exit status of `cargo xtask ci` is 1 when a check failed or, under `--label ci`, when a tool is missing, and 0 otherwise. It is 0 even when a required check is `not_run`. The exit status is not eligibility: read `disposition` in the record.
- The latest record is `target/rha/evidence.json`; `--record DIR` also keeps a timestamped copy. Commit the record for your change under `evidence/CHG-0nn/`. Records are class `local` or `ci` and advisory; see [docs/threat-model.md](docs/threat-model.md).
- Order: `cargo xtask docs`, then the lane, then commit. A record is taken on a clean tree and committed in the commit after the one it describes; commits that only add a record or a regenerated index need no record of their own, because the CI record for the pushed head covers them. A record taken on a dirty tree is kept and says so: its `snapshot_tree` names exactly what it saw, which is not the commit that followed.
- After changing `.rha/**`, `evidence/**`, or `rha-baseline.json`, run `cargo xtask docs` and commit the regenerated files. `cargo xtask docs --check` lists stale ones.
- Held-out cases (DP-1.1b, as amended in CHG-004.6): `cargo xtask corpus held-out --archive <the original tar>`, or `--cases <directory>`. The archive is verified by its bytes and the directory by re-creating the committed stream, `tar --sort=name --mtime='2026-09-20' --owner=0 --group=0 --numeric-owner -cf - -C <dir> . | sha256sum`. The command runs the accepted checker on every ready workspace, prints opaque case ids, exit statuses and finding counts, and keeps raw reports under `~/.local/state/rha/held-out/`. It grades nothing: Kennedy compares the observations with his expectations and records the per-case outcomes in the acceptance record. A run after the merge is dated as a post-acceptance observation.

## Submit and review

- Open the PR with the template. It is a review index: link the task record, the change record `docs/changes/CHG-0nn-<slug>.md`, and the evidence, and list every `not_run`, `failed`, or `inconclusive` result with its reason. Do not transcribe CI output.
- Compute counts and full hashes from their owning files or commands; never infer a full hash from a prefix. Link each human summary to the exact evidence it summarizes.
- CI (`.github/workflows/ci.yml`) reruns the lane on the PR's merge commit and uploads the record as the `rha-evidence` artifact (class `ci`). The branch must be up to date with `main` before it merges.
- The acceptance authority named in `.rha/policy.toml` `[authority]` reviews and merges. **The merge is the acceptance** (`[acceptance] by_merge`). The Executor writes `.rha/acceptances/CHG-0nn.toml` as the first commit of the next item, citing the merge commit, its tree, and the CI record for that exact revision (download it from the run on `main` and commit it under `evidence/ci/`). A contributor's "done" is a proposal, not an acceptance.
- Open questions are cited by id from `.rha/decisions.toml`, the single owner of decision-point status. A point with a default that is unanswered when its trigger arrives is recorded as decided by default; a point marked required waits. Do not restate a question that has a row.
- Review findings on an open PR are verified against the reviewed revision. Group findings that name the same failure on the same revision into one repair and batch bookkeeping corrections into one determination, not one commit each. A concrete failure becomes a `CHG-0nn.k` commit, determination first (task record and change record), then the change. An evidence record is never rewritten to describe a tree it did not see; archival records describe their preceding subject truthfully. A resolved thread reopens only on a new failure mode or an unaddressed consequence.
- Acceptor's checklist before merging: every decision point the item names has a row in `.rha/decisions.toml`; any defect to be recorded is stated so it can go under `[[defects]]` of the acceptance record; a change to a protected surface has its approval in the task record.

## Exceptions and policy changes

- Only the exception authority named in the policy issues exceptions. A contributor cannot create one by asserting low risk, and non-waivable checks (every L0 check) cannot be waived.
- Under `ECC-Solo` the issuer is also the accountable change authority, so an exception to one's own change waits `cooling_off_hours` and is appended to `.rha/exceptions.log`, which is never rewritten. An entry records scope, reason, compensating control, expiry, follow-up, and the exact subject and policy digest. The original outcome stays in the evidence record.
- A change to the policy, the workflows, the verifier (`xtask`), dependency policy, or an instruction file is a protected-surface change. Review it explicitly and do not let it approve itself: until the verifier reads the policy from the base revision, compare `verification_identity.policy_digest` with `base_policy_digest` in the record.
