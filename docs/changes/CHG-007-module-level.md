# CHG-007 (P-C): record schemas, module graph and E1

Task: [CHG-007](../../.rha/tasks/CHG-007-module-level.toml). Covers CHG-007, CHG-019.1 and E1. Plan §8.1 P-C and §8.2 W7/E1; Kennedy M2 handoff.

## Intent and scope

Complete the P-B schema follow-on before module extraction, then the registered module checks and the isolated no_std experiment. The first commit binds P-B's acceptance to its exact main CI run. The Executor is the sole branch writer; generators use staging artifacts and validators are read-only.

## Deltas

Implementation and stage evidence will be recorded here as each gate runs. No implementation stage has passed yet.

### Stage 0 baseline confirmation

[Baseline probes](../../evidence/CHG-007/stage-0-baseline-probes.txt) reproduce missing and unknown L0 evidence outcomes accepted with exit 0, alongside a valid control. The same command rejects the committed markdown corpus report as `evidence.missing_required_check`. Its accepted control also prints no unchecked-citation note despite archived raw-log paths not resolving beside this copied record or in git. No production lint/schema change existed at these observations.

DP-2.1 installed `cargo-modules 0.27.0` and `cargo-mutants 27.1.0` with `cargo install --locked`. The inventory records version-probe output and pins reinstall/CI versions; bootstrap observations retain their historical meaning.

## Verification

P-B acceptance records linted and generated docs checked before their commit. P-C stage gates and settled-head L0 remain pending.

## Decisions

`delegation-m2`, `stage-0-schema`, `dp-2.1`, and `contributing-status` in the task record quote Kennedy's authorization and state the alternatives rejected. Status lives in the decision ledger.

## Repair attempts

1. **Registration contract ambiguity, found before schemas or lint code.** Hypothesis: pooling historic record variants could accept empty provenance or reject a legitimate documentary outcome. Discriminating check: the read-only record census found compact string-source provenance, H4 fixture identity variants, and `not_part_of_this_run` in a documentary held-out field. Change: specify branches, the fixture identity exclusive choice, exact discriminator codes and outcome scope. The in-progress generator was interrupted before it wrote corpus payloads and is resumed with the corrected contract; its first log is retained. Result: contract corrected before any registration commit or implementation grading; generator validation remains pending.

## Acceptance concerns

- Held-out checks are `not_run`: Kennedy's material and runs, spec §9.14.
- Authentic remains unsatisfiable under DP-4.1; no protected-producer claim is made.
- P-C is not accepted while its PR is unmerged. P-D will stack on this head; no fictional P-C acceptance is written.
