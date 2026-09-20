# CHG-004: H4 crate-level harness

Task: [CHG-004](../../.rha/tasks/CHG-004-h4-crate-harness.toml). Plan: §6 and §8 W4.
Base: `140fdfb92d62d3481d95f3e2efc1d24a0206fcc4`, merge of PR 6.

## Intent and scope

Generate the crate-level fixtures the manifest pre-registered, run the accepted checker on them through the public command, grade every case under DP-1.1c, record the result whatever it is, and project it into the enforcement map. Result: 22/22 detected; the run is graded `failed` because C13 carries one accurate warning the registration did not name; no V is proposed (acceptance concern 1).

### Required order and approval

1. `dedc8ccda78f6b35e5e30ce2cf5db97fdede240b` records CHG-003 acceptance and archives the CI evidence for the accepted merge.
2. This commit changes EM-M03's cell association in the manifest to `cells = ["law3-d1", "law6-b3"]`, with its pinned test row and this task record.
3. Harness implementation follows those commits.

Kennedy approved the protected manifest edit in the working conversation on 2026-09-20: “I approved the EM-M03 cell edit on the manifest: cells = ["law3-d1", "law6-b3"].” Under §9.14 this corrects the test's cell attribution with explicit authority. Every other case and all grading values remain fixed (DP-1.1c).

Kennedy runs his held-out cases at acceptance (DP-1.1b); they are never read by the Executor. Accepted maturity remains his decision (DP-1.6).

## Deltas

Kennedy's explicit W4 objective requires committed generated fixtures. `cargo xtask corpus generate` materializes the registered crate cases under `xtask/tests/corpus/crate/{violations,legitimate,expected_miss}/<id>/`; outside packages remain beside the associated workspace as their declared paths require. `corpus generate --check` detects added, changed and missing input files without repairing them. Only Cargo output (target directories and lockfiles) is excluded. The committed snapshots are derivative data; the manifest remains their sole semantic owner.

The earlier crate-fixture prohibition in `corpus_manifest.rs` is replaced by the drift gate and complete `corpus_crate.rs` test, explicitly changing the assurance strategy under §9.14 as instructed. Module fixture absence is still checked. No case or grading value changes.

`cargo xtask corpus run --level crate` runs the committed cases using actual `cargo xtask architecture --manifest-path … --rules … --format json` commands and `cargo check --offline` for R01. H4 schema 2 combines all 22 detect cases while retaining checker/compiler subtotals. It records the manifest's registration identity, digest, exact correction commit, tool identity, raw outputs, witnesses and holes. Every unmatched finding remains a false alarm under DP-1.1c. `cargo xtask docs` alone projects the latest archived H4 record into the enforcement map; every module cell remains not_run until CHG-007.

The first harness packet uses the accepted checker unchanged. Its failed run is retained before the checker repair determination and CHG-004.1 repair commit. Prior draft records remain archival observations of their named revisions; the final handoff will identify the replacement records explicitly.

## Evidence

### CHG-004.1 determination, before repair

[The first clean committed-fixture run](../../evidence/h4-crate/20260920T121342Z-3c6c758911ff-1462546.json) describes `3c6c758911ffa418fbe5b2678e7b538c36a69521`. It exits 1: **13/22 detected**, **10/11 unmatched findings**, two documented holes. R01 and all 11 legitimate cases pass; C04, C05, C08, C09, C10, C11, C13, C14 and C20 fail exact witness grading.

The raw reports show those checker violations, but their structured JSON omits registered `crate`, cycle `members`, `matched_rule`, conflicting-role values, `port` or `owner` fields. Add those facts at the producer from its graph/rules observations; never derive an answer from the corpus inside checker code. The separate CHG-004.1 commit applies this repair after this determination and failed record. C13's additional `adapter.foreign_core` warning is accurate and must remain an unmatched finding under DP-1.1c; it is not suppressed to obtain a green H4 result.

### Repaired result

[Clean H4 record](../../evidence/h4-crate/20260920T121505Z-f1c2c55a5550-1465488.json) describes `f1c2c55a5550f5ee8852f83a46f03d5d2ac86292`: **22/22 detected** (21 checker and R01), **1/11 false alarms**, **2 documented holes**, exit **1**. C13 alone fails because its accurate `adapter.foreign_core` warning is unmatched under DP-1.1c. No V promotion is proposed, and `law3-d1` is downgraded. All 16 corpus tests pass, including the full public command, byte-drift negative controls, and exact-witness/report validation.

### Handoff provenance

The approval decision owns the full EM-M03 correction commit. H4 reads this fixed provenance instead of treating the newest manifest edit as the historical correction. The integration test independently loads that commit and its parent and proves that their only manifest difference is the approved EM-M03 cell edit. Fixture input hashes are labelled as generated expectations; an actual mismatch is recorded as drift and fails the harness before execution.

Outside packages remain siblings of the C19 and EM-C01 workspace roots, as `outside_crates.at` requires; every case workspace itself is at its requested category/id path. The full fixture tree and its drift check include those external packages.

### Final H4 handoff

[Final clean H4 record](../../evidence/h4-crate/20260920T121858Z-ac6f7c67af63-1471693.json) identifies `ac6f7c67af6384395a3710fc4805edd4fcf01903`. It reports 22/22 detected, 1/11 false alarms, two documented holes, 229 generated input hashes and no drift. The command exits 1; C13 is the sole failure. Every module cell remains not_run under CHG-007; no V promotion is proposed (DP-1.6). Kennedy alone runs DP-1.1b.

| Case | Actual outcome |
| --- | --- |
| C01 | Detected; passed |
| C02 | Detected; passed |
| C03 | Detected; passed |
| C04 | Detected; passed |
| C05 | Detected; passed |
| C06 | Detected; passed |
| C07 | Detected; passed |
| C08 | Detected; passed |
| C09 | Detected; passed |
| C10 | Detected; passed |
| C11 | Detected; passed |
| C12 | Detected; passed |
| C13 | Detected; failed: unmatched adapter.foreign_core warning |
| C14 | Detected; passed |
| C15 | Detected; passed |
| C16 | Detected; passed |
| C17 | Detected; passed |
| C18 | Detected; passed |
| C19 | Detected; passed |
| C20 | Detected; passed |
| C21 | Detected; passed |
| L01 | No alarm; passed |
| L02 | No alarm; passed |
| L03 | No alarm; passed |
| L04 | No alarm; passed |
| L05 | No alarm; passed |
| L06 | No alarm; passed |
| L07 | No alarm; passed |
| L08 | No alarm; passed |
| L09 | No alarm; passed |
| L10 | No alarm; passed |
| L11 | No alarm; passed |
| EM-C01 | Documented hole; passed with no findings |
| EM-C02 | Documented hole; passed with no findings |
| R01 | Detected E0603; passed (cargo exit 101) |

### Local L0 handoff

[Clean local L0 record](../../evidence/CHG-004/20260920T122054Z-bef220d57566.json) identifies `bef220d57566868d5407fb03b9e19389b04c1d5f`: all eight checks passed and nextest selected 141 tests, including the complete committed corpus and drift controls. Authentic remains false under the bootstrap policy; the separate H4 run remains failed because of C13.

| Check | Actual outcome |
| --- | --- |
| L0.fmt | passed |
| L0.clippy | passed |
| L0.nextest | passed |
| L0.doctest | passed |
| L0.architecture | passed |
| L0.deny | passed |
| L0.machete | passed |
| L0.typos | passed |

### CHG-004.3: shallow-checkout provenance determination

[CI run 35510429218](https://github.com/kmosoti/recursive-hexagonal-architecture/actions/runs/35510429218), retained [unchanged](../../evidence/ci/35510429218.json), failed only L0.nextest: 140 tests passed and the full-corpus test failed when `15d916a` could not be resolved. The protected workflow uses fetch-depth 2; the local checkout had those older objects, so its lane passed.

Historical citations must not require history downloads to grade the current fixtures. Read the original revision from CHG-002's existing acceptance record, and the correction from its approval decision. The decision now also records the parent and corrected manifest byte digests, measured from the actual correction commit and its parent before this repair. The test reconstructs only the approved cell change and compares both byte digests. Under §9.14 this replaces an unconditional deep-history dependency with a fixed provenance check that also runs in CI; it skips no assertion about the cases or their outcomes. The protected workflow remains unchanged. New clean H4 records also use the requested timestamp–revision filename without a process-id suffix; create-new semantics still prevent overwrites.

### Delivery H4 after the CI repair

[Clean delivery H4](../../evidence/h4-crate/20260920T122638Z-eec489c71f25.json) identifies `eec489c71f259423bb4cc75a5650ecaa8c10c0c3` and supersedes the earlier H4 records for this handoff. Every case has the same actual outcome as the full table above: 22/22 detected, 1/11 unmatched findings, two documented holes, C13 alone failed, exit 1. The registration and correction identities are cited from their owning records and verified byte digests; neither requires older Git objects on the CI runner.

[Delivery local L0](../../evidence/CHG-004/20260920T122732Z-885a709f7f49.json) records the settled CI repair at `885a709f7f49`: all eight checks passed, 141 nextest tests. Earlier records, including failed CI run 35510429218, remain unchanged and are superseded by these delivery records for this handoff.

### CHG-004.4: make-ready review

Verified on a second machine at `2ad75c8`: `cargo xtask corpus generate --check` reports 229 inputs and no drift; `cargo xtask docs --check` reports every generated file current; 141 tests pass; `cargo xtask corpus run --level crate` reproduces 22/22 detected, one unmatched finding, C13 the sole failure, exit 1.

**CI**: run [35510728858](https://github.com/kmosoti/recursive-hexagonal-architecture/actions/runs/35510728858) on the draft's head `2ad75c8`, copied as [`evidence/ci/35510728858.json`](../../evidence/ci/35510728858.json): all eight `passed`, 141 tests; its subject tree `a6012b56` equals the head's tree. Determinations 1 to 5 are in the task record's decision `make-ready-chg-004.4`: two journals removed, the task record reshaped, this section and the concerns below added, the CI record committed. The local record for the make-ready commit follows.

## Acceptance concerns

1. **C13 is your decision.** The harness applied DP-1.1c as written: C13 is detected, and the checker's accurate `adapter.foreign_core` warning about `adapter-x -> core-b` is an unmatched finding, so the run is `failed`, `law3-d1` is downgraded, and no V is proposed. The warning is not over-eager: plan §5 listed `adapter.foreign_core` (warn) before any data, and C13's seeded shape, an adapter depending on a core whose port it does not implement, is exactly its trigger. The registration is what did not anticipate it. Three ways to close this, all yours: (a) amend the registration, a protected edit recorded on the ledger first as a DP-1.1c amendment and disclosed as made after the data: give C13 `expected_findings = [{ rule = "adapter.foreign_core", crate = "adapter-x", to = "core-b" }]` and extend `[grading] extra_findings` so a finding matching an entry of a case's `expected_findings` is a registered fact, neither a detection nor a false alarm; a CHG-004.5 re-run is then recorded beside this one, and a V proposal for the crate level becomes honest with that disclosure; (b) re-seed C13 so the adapter's other dependency is external rather than a core, which also changes a pre-registered shape after the data and loses the two-fact case; (c) leave the result as failed, with `law3-d1` downgraded until a later item. The supervising session recommends (a).
2. **Four records cite revisions absent from this branch.** The draft's history was rewritten before its first push; the two CHG-004 records for `762ab47` and `540ce6a` and the two H4 records for `defbfb7` and `b8c3785` describe commits nobody can check out. They are kept as observations, not deleted, and they are not the evidence for this candidate; the delivery records above are.
3. **The integration test pins the failed outcome.** `xtask/tests/corpus_crate.rs` asserts `failed_cases == ["C13"]`, so L0.nextest guards the observation, not a pass. Decision (a) or (b) changes that assertion in the same commit as the amendment.
4. **Held-out cases (DP-1.1b) and the maturity proposal (DP-1.6) are yours.** The record's `held_out` entry is `not_run` with the reason; the maturity row proposes nothing beyond I.
5. **Two journals were removed again** (CHG-004.4 determinations 1 and 2). If the executor's harness needs a place for its own lessons, it is outside `docs/`, which the wiki renders.
