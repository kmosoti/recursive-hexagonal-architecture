# Threat model and deviations

This repository adopts the default threat model of spec §11.0 and records its deviations here, as §11.0 asks. The profile is **`ECC-Solo`**; it is strictly weaker than separated duties, and every conformance claim made from this repository says so.

## Authority

| Item | This repository |
| --- | --- |
| Accountable authority | `human:kennedy`: Planner, Integrator, Acceptor, and exception authority. |
| Executor | `agent:claude-opus-5`, accountable to `human:kennedy` (contribution mode `agent`). A separate agent instance does not add an accountable principal (§11.6.1). |
| Why `ECC-Solo` | One human directs the work and controls acceptance and exceptions, so separation of duties is nominal (§11.7.6). |
| Substitute controls | Exceptions to Kennedy's own changes wait a cooling-off delay (`cooling_off_hours` in `.rha/policy.toml`) and are appended to [`.rha/exceptions.log`](../.rha/exceptions.log). The repository is public, so the log is public. |
| Append-only log | Enforced by review and visible in git history, not by a technical control. |

## Evidence classes

Every evidence record is class `local` or `ci`. None is `protected`:

- `cargo xtask ci` is candidate code. The CI workflow runs the pull request's own `xtask`, so a candidate can change the program that writes its evidence (§11.5).
- The policy is read from the candidate's working tree, not from the base revision (`Policy(b)`, §11.7.6). Each record reports both `policy_digest` and `base_policy_digest` so a reviewer can compare them.
- No producer is authenticated: `trusted_producers` is empty until the verifier of CHG-020 exists. The `Authentic` predicate is therefore false in every record, and `disposition.eligibility` is `blocked` for that reason alone at minimum.
- Until then, acceptance is Kennedy's decision after reading the record's predicates. It is not `MergeAllowed` of §11.7.6.

## Adversaries (§11.0) and the controls actually present

| Adversary | Controls present at CHG-000 | Missing or weaker than §11.0 |
| --- | --- | --- |
| A1 fallible contributor | Five outcome states; `passed` needs a validity criterion; `not_run` for missing tools; the §9.14 weakening rule in CONTRIBUTING; the policy drift test. | No protected rerun. Held-out cases arrive with DP-1.1 and DP-1.4. Mutation arrives with CHG-014. |
| A2 steered agent | Text is data (AGENTS.md reminder). The CI token is read-only, and checkout does not persist credentials. | The Executor's permissions are the harness permission mode on Kennedy's account, not a task-scoped grant. |
| A3 malicious external contributor | Pull requests from forks run with a read-only token and no secrets. Branch protection requires the `ci` status and applies to admins. Protected surfaces are listed in the policy. | The candidate's `xtask` and policy decide its own CI outcome; only review of protected surfaces and the digest comparison catch a self-approving change. |
| A4 compromised dependency or action | Actions pinned by commit SHA. Tools installed with `taiki-e/install-action`, which verifies checksums [R43]. `cargo deny`, Dependabot, and no secrets in any workflow. | Tool versions are pinned; their artifacts are not independently re-verified. |
| A5 administrator, CI provider, or collusion | Out of scope, as in §11.0. | – |

## Trust assumptions

GitHub enforces branch protection (required status `ci`, branch up to date, admins included, no force-push) and isolates jobs as documented [R61]. The weakest assumption is an attentive Acceptor. The review-efficacy audit of §11.4 needs reviewers other than the author and is deferred (plan §10).
