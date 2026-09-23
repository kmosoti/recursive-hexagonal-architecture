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


### Stage 0 implementation and determinations

Seven Draft 2020-12 schemas are deterministically translated from the registered inventory and compiled offline by jsonschema 0.57.0. The lint selects record families from parsed data, applies structural rules, preserves the previous semantic codes, and explicitly lists unchecked citations. H4, markdown and H5 reports use their own shapes. Citation lookup remains beside the record or at its git subject, with traversal/symlink escapes refused. The library has no HTTP/file-reference resolving features enabled. CONTRIBUTING's stale capability table is corrected under `contributing-status`.

[Targeted results](../../evidence/CHG-007/stage-0/registered-lint-and-schema.txt): 1,497/1,497 structural recipes and 80/80 original lint fixtures match their exact registered code sets; CLI controls cover missing/unknown outcomes, all three report families, comments and citation resolution. [Verifier results](../../evidence/CHG-007/stage-0/registered-verifier.txt): all original 152 and supplementary 188 fixtures match. No verifier production behavior changed. These logs are working-tree observations; the settled packet's revision-bound L0 record follows at completion.

**Repair attempt 2, integration of write-tier proposals (§11.7.10).** Hypothesis: proposal wiring and harness construction, rather than registered requirements, explained initial failures. Discriminating checks: [compile errors](../../evidence/CHG-007/stage-0/proposal-compile-failure.txt) identified Option flattening and borrowed/owned family types; [harness failures](../../evidence/CHG-007/stage-0/harness-first-failures.txt) identified missing miniature inventory counts and an incorrect byte-lexical ordering assumption; [recipe failure](../../evidence/CHG-007/stage-0/recipe-null-failure.txt) showed explicit JSON null collapsed into an absent Option. A CLI test also invented a full-SHA filename; its failing output was observed but that temporary log was overwritten, so no retained full log is claimed. Changes: correct types and preserve configuration errors; give the miniature inventory its actual count/length; compare path components as the registered Python Path sort does; distinguish present null from absent value; select report fixtures from SOURCE-INDEX. Result: all targeted checks pass. This corrects newly authored harnesses, not registered data or expectations (§9.14).

**Checker discrimination.** Temporarily returning no schema issues makes the [CLI regression fail](../../evidence/CHG-007/stage-0/negative-validator-control.txt), as required. Temporarily replacing a generated schema with an empty object makes [the projection check fail](../../evidence/CHG-007/stage-0/negative-codegen-control.txt). Both injected defects were restored, then the targeted checks passed. The inventory checker likewise rejects changed/deleted payloads with a valid positive control. No injected defect is committed.

**Assurance limits.** Structural compatibility does not establish authenticity, referential completeness, corpus execution or acceptance. Unobserved item domains and declared dynamic maps remain as registered, and historical record defects remain disclosed rather than rewritten. The original 80/152 corpus grading and the module manifest are untouched.


**Repair attempt 3, first full stage gate.** The [failed nextest run](../../evidence/CHG-007/stage-0/first-stage-gate-failure.txt) ran 192/223 tests (191 passed, one failed; 31 not run after fail-fast). Hypothesis: CHG-002's `module_fixtures_wait_for_chg_007` guard had reached its expiry. The failing assertion requires the module corpus directory to be absent, while this packet must commit it before extraction. Change: replace absence with exact authored-case completeness and per-case equality to the frozen manifest, retaining every grading/count pin and the separate payload digest checks. This is the approved CHG-007 requirement advancing (§9.14), as that test's previous CHG-003 transition already documents; no expected outcome is relaxed. Result: the repaired stage gate follows below.

**Stage 0 gate:** [cargo nextest run](../../evidence/CHG-007/stage-0/stage-gate-nextest.txt) executed all 223 tests, all passed, none skipped. `cargo xtask docs --check` passed. Targeted clippy completed with advisory warnings under §6.4; no deny-level failure. Stage 0 is complete; module extraction, rule integration, module grading, site validation and E1 remain.
