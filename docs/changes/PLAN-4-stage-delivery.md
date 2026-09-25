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
