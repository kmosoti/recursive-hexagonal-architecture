# PLAN-3: align the kickoff prompt and M6 with the ledger

**Intent.** A prompt audit on 2026-09-23 (target model Claude Opus 5.5) found the plan's packet kickoff prompt and its method rule M6 restating reviewer decisions that the ledger has since changed:
- Codex's role: a validator, not a generator (DP-5.4).
- The reviewer: `gpt-6-sol` at medium, with `gpt-6-astra` only after asking (DP-5.4 amendment).
- The merge condition: an Opus 5.5 and a GPT-6 approval of the same head.
- The gate rule: decide under Kennedy's delegation of 2026-09-22.

An executor that followed the plan literally would use the wrong reviewer, role and effort, and escalate what it was told to decide. DP-5.4 says the plan text follows at its next revision; this is that revision.

**Deltas** (`docs/plan/IMPLEMENTATION-PLAN.md` only):

| Line | Finding | Confidence | Change |
|---|---|---|---|
| 7 | F1 | high | Reviewers are now named by reference to DP-5.4, not pinned. |
| 7 | F3 | medium | Effort is left to the session configuration instead of "largest … available". |
| 14 | F4 | medium | Gates are decided under the delegation; held-out material and Astra stay Kennedy's. |
| 16 | F5 | medium | Two approving reviews of the same head. |
| 67 | F2 | high | M6 matches DP-5.4 and the merge rule. |

**Not changed.**
- The frozen plan copies inside the registered corpus inputs; their digests are evidence.
- F6 (`crates/site/AGENTS.md:7`, an illustrative list of extractor holes), a low-confidence flag.

**Checks.** No test or tool reads the plan's bytes. The only reference is a registration's historical digest, which remains correct as history. `cargo xtask scope` and the L0 lane are in `evidence/PLAN-3/`.

**Acceptance concerns.**
- The GPT-6 half of the review cannot run until the Codex quota resets on 2026-09-29.
- The acceptance record owed for CHG-007 (PR 18) is assigned to P-D by its resume point, not written here.
