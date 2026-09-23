# CHG-007 (P-C): record schemas, module graph and E1

Task: [CHG-007](../../.rha/tasks/CHG-007-module-level.toml). Covers CHG-007, CHG-019.1 and E1. Plan §8.1 P-C and §8.2 W7/E1; Kennedy M2 handoff.

## Intent and scope

Complete the P-B schema follow-on before module extraction, then the registered module checks and the isolated no_std experiment. The first commit binds P-B's acceptance to its exact main CI run. The Executor is the sole branch writer; generators use staging artifacts and validators are read-only.

## Deltas

Implementation and stage evidence will be recorded here as each gate runs. No implementation stage has passed yet.

### Stage 0 baseline confirmation

[Baseline probes](../../evidence/CHG-007/stage-0-baseline-probes.txt) reproduce missing and unknown L0 evidence outcomes accepted with exit 0, alongside a valid control. The same command rejects the committed markdown corpus report as `evidence.missing_required_check`. Its accepted control also prints no unchecked-citation note despite archived raw-log paths not resolving beside this copied record or in git. No production lint/schema change existed at these observations.

DP-2.1 installed `cargo-modules 0.27.0` and `cargo-mutants 27.1.0` with `cargo install --locked`. The inventory records version-probe output and pins reinstall/CI versions; bootstrap observations retain their historical meaning.

### Module registration review (before implementation)

The independent generator's first proposal preserved an M01 ambiguity: the undeclared reverse edge accompanies its cycle. The pre-code module diagnostic contract resolves it by retaining undeclared edges as structured cycle subfacts, without changing the frozen witness or grader. A read-only review found the optional random-rule oracle still emitted those subfacts as separate findings. The generator is revising that proposal before registration; its original output digest and correction prompt are retained in generation provenance. Extraction reference outputs remain unchanged. M05 depth counts canonical path segments including the crate root; M21's legal Rust 2015 case is disclosed alongside Rust 2021 supplementary uniform-binding coverage.

## Evidence

P-B acceptance records linted and generated docs checked before their commit. P-C stage gates and settled-head L0 remain pending.

## Decisions

`delegation-m2`, `stage-0-schema`, `dp-2.1`, and `contributing-status` in the task record quote Kennedy's authorization and state the alternatives rejected. Status lives in the decision ledger.

## Repair attempts

1. **Registration contract ambiguity, found before schemas or lint code.** Hypothesis: pooling historic record variants could accept empty provenance or reject a legitimate documentary outcome. Discriminating check: the read-only record census found compact string-source provenance, H4 fixture identity variants, and `not_part_of_this_run` in a documentary held-out field. Change: specify branches, the fixture identity exclusive choice, exact discriminator codes and outcome scope. The in-progress generator was interrupted before it wrote corpus payloads and is resumed with the corrected contract; its first log is retained. Result: contract corrected before any registration commit or implementation grading; generator validation remains pending.

## Acceptance concerns

- Held-out checks are `not_run`: Kennedy's material and runs, spec §9.14.
- Authentic remains unsatisfiable under DP-4.1; no protected-producer claim is made.
- P-C is not accepted while its PR is unmerged. P-D will stack on this head; no fictional P-C acceptance is written.


### Independent module corpus registered before extraction

The separate `codex exec` generator supplied 256 seeded legal source maps and an independent lexical reference extractor with their expected edges, plus 25 compiling headline fixtures. The parent checked every payload hash and the diagnostic-contract hash before committing [the registration](../../xtask/tests/corpus/module/registration.toml). Seed, model, effort and both prompt digests are in the task's generation record. The first proposal remains identifiable in the corpus provenance; the correction preserves all 128 D4 facts (64 independent findings, 64 cycle subfacts). No real extractor was written or run before this registration.

The artifact self-checks record 512 rustc legality checks, 25 offline Cargo checks, 14 hand-derived reference checks, 4,096 supplementary graph checks and 641 byte-identical regeneration comparisons. These validate corpus construction, not a production checker. The frozen M/L-M/EM-M manifest remains the headline oracle.

Reproduction of the immutable generation package uses its original staging layout: copy `xtask/tests/corpus/module` to `target/m2/module-generation`, and the two committed module prompts to `target/m2/module-generator-prompt.md` and `target/m2/module-generator-correction.md`, then run its README commands. The scripts never rewrite the committed registration in that workflow.


### Stage 0 registrations

The [record-shape registration](../../xtask/tests/corpus/record-schema/registration.toml) binds 119 committed source snapshots (116 supported records plus three excluded BDR reports), the mechanically derived inventory, and 1,497 exact structural recipes. Historical variants remain explicit; no historical record is rewritten. A separate read-only audit independently matched all 1,497 structural expectations before implementation.

The [verifier-shape supplement](../../xtask/tests/corpus/verifier-shape/registration.toml) binds 188 fixtures: 153 malformed cases and 35 well-formed controls/predicate cases. Its construction is independent of model code. The original 152 verifier cases and 80 lint cases remain unchanged. This supplement grades the disclosed after-data amendment, not a claim of pre-registration before the P-B verifier existed.

During pre-registration, the generator proposed precise leaf-path diagnostic labels where the old contract had no wire spelling. §4.1 now mechanically records P-B's existing grouped labels; 91 proposed diagnostic strings were corrected before commit, preserving every predicate expectation and precise offending JSON Pointer. The three prompts and their digests, interrupted attempt and failed inspection commands are retained. This compatibility clarification avoids changing existing verifier behavior for a cosmetic diagnostic choice.
