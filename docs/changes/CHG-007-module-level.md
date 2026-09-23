# CHG-007 (P-C): record schemas, module graph and E1

Task: [CHG-007](../../.rha/tasks/CHG-007-module-level.toml). Covers CHG-007, CHG-019.1 and E1. Plan §8.1 P-C and §8.2 W7/E1; Kennedy M2 handoff.

## Intent and scope

Complete the P-B schema follow-on before module extraction, then the registered module checks and the isolated no_std experiment. The first commit binds P-B's acceptance to its exact main CI run. The Executor is the sole branch writer; generators use staging artifacts and validators are read-only.

## Deltas

Implementation and stage evidence will be recorded here as each gate runs. No implementation stage has passed yet.

## Verification

P-B acceptance records linted and generated docs checked before their commit. P-C stage gates and settled-head L0 remain pending.

## Decisions

`delegation-m2`, `stage-0-schema`, `dp-2.1`, and `contributing-status` in the task record quote Kennedy's authorization and state the alternatives rejected. Status lives in the decision ledger.

## Repair attempts

None yet.

## Acceptance concerns

- Held-out checks are `not_run`: Kennedy's material and runs, spec §9.14.
- Authentic remains unsatisfiable under DP-4.1; no protected-producer claim is made.
- P-C is not accepted while its PR is unmerged. P-D will stack on this head; no fictional P-C acceptance is written.
