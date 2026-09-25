# PLAN-4: revision 3, stages as the delivery unit

**Intent.** A study of the program's own history asked whether the division of work produces deliverable, testable stages that inform the next one (`~/experiments/studies/2026-09-rha-stage-division`, REPORT.md, verified independently). Its results are descriptive: the thresholds were set by the session that already knew most outcomes. With verifier-corrected coding:
- **Deliverable:** 0.375 of CHG-level stages were accepted with CI in their own PR. At packet level the figure is 0.93. P-D's first six stages ran 48 commits with no PR and no CI run.
- **Feedback:** 0.40 of stages had CI before the next stage began.
- **Testable:** about 2 P1/P2 contract defects per P-D feature stage passed every registered grader.
- **Informs:** 0.64 of transitions carried a lesson forward, all by hand. One lesson was lost (CHG-009 to CHG-010).

**Deltas** (`docs/plan/IMPLEMENTATION-PLAN.md` only):

| Where | Change |
|---|---|
| M2 | A stage that delivers a feature, measurement or harness is merged on its own, with CI on its PR; the packet keeps one task and change record |
| M6 | An agy teamwork run used only for verification (everything but its probes frozen) is a second-family review while GPT-6 is unavailable; agy implements with Boost or a single agent |
| M11 (new) | A stage's close lists "For the next stage"; the next contract commit cites or rejects each item |
| M12 (new) | A revision differential against the parent, with a negative control, is registered at contract time; goldens become secondary |
| M13 (new) | Stages over 200 cases, 3 packages or 3 production crates are split before code |
| P-D | The completed stages merge as one catch-up PR; feature 5 and stage 3 follow as stage PRs; change-spread reports the plan's original prediction accuracy (2 of 5) as its headline |

**First commit.** `.rha/acceptances/PLAN-3.toml`: merge ac1e460, main CI run 36008186429 (all eight L0 checks passed, 290 tests). The record lint accepts it.

**Not changed.** The frozen plan copies in the registered corpus inputs. Merged packets and P-D's completed stages keep their history.

**Acceptance concerns.** No GPT-6 review is possible until 2026-09-29. The study's pre-registration was weak, and the report says so.

**Review of cbe2d3b (separate Claude verifier): REQUEST_CHANGES; determinations.** Every value in the PLAN-3 acceptance record was recomputed and found correct; scope, lint, docs and typos passed. Findings:
- **P2, confirmed: the PLAN-3 record had the retired kind.** It used `controlled_transition` and had no predicates, while `[acceptance.bootstrap]` has required `bootstrap_acceptance` with `[[disposition.predicates]]` since 5a768af. Repair: kind and all four predicates, as in CHG-007.toml; lint accepts it. The lint does not check this rule, which is a gap in the lint and a follow-up.
- **P2, confirmed: M6 cited a decision absent from main and an agy rule with no recorded source.** Repair: M6 now names decisions agy-verification-second-family and agy-teamwork-verification-only (Kennedy's words), which PR #21 records. PR #21 therefore merges before this PR, and this branch merges main before its own merge.
- **P2, confirmed: revision 2 text still said one PR per packet** (provenance, kickoff prompt, M2 base text, M5, §2.3). Repair: each passage now states the revision 3 rule, and the provenance says revision 3 wins where old text remains.
- **P3, confirmed: M11 to M13 came before M10.** Repair: reordered. The change-spread headline that line 440 promises is added in PR #21 (5e1dd51).

**Re-review of 34981ab: REQUEST_CHANGES; determinations.** The earlier findings were confirmed repaired. New findings:
- **P2, confirmed: kickoff step 5 still set one L0 record, one review pair and one PR per packet.** Repair: step 5 now works per stage PR.
- **P3, confirmed: M6 opened with "when the packet is ready", and the risk table said "one L0 record per packet".** Repair: both now say per stage PR.

**Merge order and P-D's acceptance.** PR #21 (P-D catch-up) merged first, at abd1f847821e2b211563b2928f51f0638578242e, because M6 here cites decisions it records. This branch then merged main (036ed82), with the two generated indexes regenerated. It also carries `.rha/acceptances/CHG-008.toml`: merge abd1f847821e2b211563b2928f51f0638578242e, main CI run 36150611033 (all eight L0 checks passed, 407 tests), and the two review verdicts. That record is not this item's first commit, because P-D merged after PLAN-4 opened; the order is disclosed here. PLAN-4's own acceptance is the first commit of the next PR.
