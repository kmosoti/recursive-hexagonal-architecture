# CHG-019 (P-B): record lint and the §11.7.6 verifier

Task record: [`.rha/tasks/CHG-019-trust.toml`](../../.rha/tasks/CHG-019-trust.toml). Plan: §8.1 P-B; §8.2 W15 and W16. Base: `72c5cfd`.

## Intent and scope

Packet P-B covers CHG-019 and CHG-020. It builds the record lint and the acceptance model of spec §11.7.6, so that `Authentic` can hold once keys exist and acceptances stop being bootstrap exceptions (lesson L10). It runs concurrently with P-A, which depends on nothing here (task record decision `concurrent-with-p-a`).

## Deltas

### Stage 1, contract and registration (before any code)

- `docs/architecture/verifier-contract.md` fixes the fixture interface: obligations, predicates, exceptions and MergeAllowed as executable terms of §11.7.6, plus the lint's reason codes. Choices the spec leaves open are marked and apply uniformly.
- A separate Codex session (`gpt-6-astra`, extra-high) generated **152 verifier fixtures** and **80 record fixtures** against that contract, while no lint or verifier code existed. Each corpus has a `registration.toml` with the prompt digest and tree digest, and `xtask/tests/corpus_trust.rs` pins both. The generator wrote down every case the contract left undetermined, including schema requiredness, nested unknown keys, comparison intervals, and the permission, graph and team rows of §11.7.9. None were invented.

### Stage 3, the verifier (`tools/rha-verifier`)

`evaluate(fixture)` computes `R_eff` (triggers, total classification, local policies that only add, the strictness preorders, the join and its conflicts), the four predicates, `Eligible`, `ValidException` with the ECC-Solo cooling-off path, and `MergeAllowed`. **It decides all 152 registered fixtures exactly as registered on its first run**, every key of every expected object included. It is a `tool` by metadata. `Cargo.toml` gains the plan's `tools/*` workspace member (decision `workspace-tools-member`).

**Differential and model properties (§2.4 rule 4; W16).** A separate Codex session wrote an independent reference verifier, `tests/reference/mod.rs`, from the contract alone. It was told not to read this crate's code or the corpus. `model.rs` had been committed 6.5 minutes earlier, so the claim is independence by instruction, not by order (corrected after the Opus 5.5 review). The two implementations agree on all 152 fixtures and on 512 random mutations: surface, outcomes, test counts, duplicate entries, integrity, time, revocation, parameters, acceptor and producer. Property tests over the same mutations check Lemma 1 (obligations only grow), Lemma 3 (evidence cannot choose the obligations) and Proposition 2 (no silent pass). `rha-verifier corpus <dir> <out>` writes the H5 record to `evidence/h5/`.

### Stage 2, the record lint (`cargo xtask rha lint`)

The lint implements the contract's reason codes over evidence (JSON), task and acceptance (TOML) records. Its field rules are stated at the top of `xtask/src/record_lint.rs`. The first run graded **77 of 80** as registered. All three disagreements were defects in the lint:

- **R025:** it let `unknown` pass as any digest, including a policy digest. It is now accepted only inside `instruction_sources`, where spec §11.6.5 allows it for messages with no file.
- **R037:** it accepted the offset `+25:00`. Offsets and date and time fields are now range-checked.
- **R038:** it skipped a non-numeric per-check `started_at`. In tool-written evidence every `_at` field is now checked.

After the fixes it grades **80 of 80**.

**Run over this repository's own records**, the lint accepts 62 and rejects 9. These are real defects in merged history, disclosed here and not rewritten:

| Records | Reason | Cause |
| --- | --- | --- |
| `.rha/acceptances/CHG-002.toml`; `.rha/tasks/CHG-004.6-c13-registration.toml`; two `evidence/CHG-004.6/` records that copied the latter's provenance | `revision.malformed` | abbreviated revisions (`6518f6a89af5`, `5ef607c`) where a full identity is required |
| `evidence/CHG-000/`, `evidence/CHG-001/`, three early `evidence/ci/` records | `digest.malformed` | `base_policy_digest = "absent"` where the field needed `null`, from W0 when no base policy existed |

### Review round 1 (Codex, `gpt-6-astra`, extra-high, read-only) on `edf4ed1`, determinations

REQUEST_CHANGES: seven P1 and one P2.

1. **P1, fractional seconds were truncated. Confirmed.** A `logged_at` or `issued_at` with `.900` validated an exception 0.9 s early. Repair: timestamps are compared in nanoseconds.
2. **P1, integer parameters were compared as `f64`. Confirmed.** Above 2^53, `n = 9007199254740992` satisfied a required `9007199254740993`, and a join could drop the stricter value. Repair: two integers compare exactly, and floating point is used only when either side is fractional.
3. **P1, Lemma 1 was vacuous for a missing obligation. Confirmed.** Repair: the test derives the triggered root checks independently and asserts each is present in `R_eff`.
4. **P1, Proposition 2 was vacuous for a wrongly passed check. Confirmed.** Repair: the test recomputes every check's pass (outcome and kind validity) itself. When merge is allowed, either all pass or the exception waives every one that does not.
5. **P1, the lint accepted 30 February. Confirmed.** Repair: days in month, with leap years.
6. **P1, the lint panicked on a non-ASCII timestamp suffix. Confirmed.** Repair: the offset is parsed from bytes, never by slicing a string at a byte index.
7. **P1, the `Cargo.toml` edit had no quoted approval. Confirmed.** Kennedy was asked and approved all three plan-derived protected edits (decision `protected-edits-quoted`). P-A's task record quotes the same answer.
8. **P2, generation provenance for the Executor's own code. Declined.** Rule 1 governs artifacts from separate generator sessions; the Executor's work is covered by `[[provenance.sessions]]`. Revision 3 of the plan should say so plainly.

The mutation generator also gains fractional timestamps and integer parameters near 2^53, so the differential test now reaches findings 1 and 2.

### Review round 2 on `10273a1`, determinations

Findings 3 to 7 were confirmed resolved. Two were incomplete, one was new, and one determination was overturned:

- **Finding 1, incomplete: digits past the ninth were dropped. Confirmed.** A timestamp with more than nine fractional digits is now refused rather than rounded, so an exception fails closed.
- **Finding 2, incomplete: mixed integer and decimal at 2^53. Confirmed.** JSON parsing already rounds such a decimal to `f64`, so it cannot be ordered exactly. The comparison is now incomparable in that range, which is a conflict or a non-refinement. There is a regression test.
- **New P1: Proposition 2 had lost its validity assertions in round 1's rewrite. Confirmed.** Both the validity assertions and the independent pass checks now stand.
- **New P2: a wrong revision in the round 1 heading. Confirmed.** Corrected to `edf4ed1`.
- **Finding 8, overturned.** The reviewer showed that plan §2.4 rule 1 names generated code explicitly, so the declination did not hold under the current text. The Executor's work now has a `[[provenance.generations]]` entry. Its prompt is Kennedy's quoted instruction in the task record's `authority` decision, and its digest is the sha256 of that exact quoted string. P-A's task record gets the same entry.

### Automated review threads on PR 17 (`chatgpt-codex-connector`, on `9bcffad`), determinations

1. **`cargo xtask rha lint` with no operands exits 0 (P2). Confirmed.** Clap gives an empty `Vec`, the loop runs zero times, and the command reports success having validated nothing. Repair: the operand is required, so a bare invocation is a usage error (exit 2, xtask's usage code). Discovering every repository record by default was the alternative. It was not chosen because it would change the command's meaning, and a default run would fail on the nine historical defects this packet discloses without rewriting.
2. **`rha-verifier corpus` passes an empty or partial directory (P1). Confirmed.** `passed == cases.len()` holds at 0/0, and nothing tied the directory to its registration, so a wrong path or deleted fixtures produced a `passed` H5 record. Repair: before deciding anything, the runner reads the directory's `registration.toml` and recomputes its `tree_sha256`, the same sorted `<path> <sha256>` lines as `xtask/tests/corpus_trust.rs`. A missing registration, a digest mismatch or zero fixtures is a refusal (exit 2) and writes no record. The record now carries the registered digest. Discriminating checks: integration tests on an empty directory, a copy with one fixture deleted, and the registered corpus. The first two fail on `9bcffad`, which exits 0 after writing a `passed` record, and are refused after the repair; the third passes 152/152.

### Conformance validation of the repair range `9bcffad..8db0531` (Codex as validator, DP-5.4), determinations

Verdict on `8db0531`: does not conform, two findings, two questions.

1. **DP-4.1 and DP-5.3 decided in the task record but not on the ledger (P1). Confirmed.** Repair: DP-4.1's row is updated and a DP-5.3 row is added, both citing the task record decisions.
2. **`docs/evidence/index.md` stale (P1). Confirmed.** The Executor ran the regeneration with an unsupported flag and hid its error. Repair: regenerated; `docs --check` is run before every push.

Questions answered. *Before-repair evidence:* the new tests ran against the unrepaired code in the working tree, before the repair commit, with these results: `rha-verifier --test cli` gave `FAILED. 1 passed; 2 failed` (the two refusal tests failed, and the registered corpus passed), and `xtask --test record_lint_cli` gave `FAILED. 0 passed; 1 failed`. After the repair both passed. The output was observed in the session and not retained as a file, so it is recorded here. *Refuted thread:* none on this pull request; the prompt was generic.

### Conformance validation of the merge with main (Codex as validator), determinations

Verdict on `9c139a6`: does not conform, one finding, one question.

1. **P-A's acceptance record is missing (P1). Confirmed.** Decision `concurrent-with-p-a` assigns it to the item that merges after P-A, which is this one. Repair: `.rha/acceptances/CHG-005.toml` binds the merge `dc564ea` to CI run 35799644918 (`evidence/ci/35799644918.json`, all eight passed, 193 tests), and `docs/maturity.md`'s Accepted column records DP-1.6. It is late. It should have been this packet's first commit after the merge; the record says so, and history is not rewritten. `cargo xtask rha lint` rejected the first draft for an abbreviated `subject_revision`, the same defect as the nine historical ones. It was corrected before commit.
2. **Question: is the lint limited to L0 evidence envelopes? Yes, by the contract.** Every JSON operand is read as an L0 evidence record (`verifier-contract.md` §2), and no contract is registered for corpus reports such as `evidence/md-corpus/`, so linting one reports `evidence.missing_required_check`. This is a disclosed limit, for the schema-contract follow-on.

Also found at the merge: P-A's scope guard, run here for the first time, flagged `Cargo.toml` missing from this record's `scope_globs` (approved in `protected_scope`); added, decision `scope-cargo-toml`.

### Approving reviews on `522445f` (Kennedy's rule of 2026-09-22: an Opus 5.5 agent and a GPT-6 agent), determinations

**GPT-6 (`gpt-6-astra`, extra-high, validator role): REQUEST_CHANGES.**

1. **`.rha/acceptances/CHG-005.toml` lacks `[[disposition.predicates]]` (P1). Confirmed.** `[acceptance.bootstrap]` requires the failed predicates with their reasons in that table, as CHG-004 and CHG-004.6 have. The repair adds all four predicates. `.rha/acceptances/PLAN-2.toml` on main has the same gap; it is a historical record and is disclosed here, not rewritten.

**Opus 5.5 (an independent reviewer agent): APPROVE, with four findings and five questions.**

1. **The verifier fails open on malformed fixtures (P2). Confirmed by the reviewer's probe crate.** A missing `revoked` reads as "not revoked". A missing `accountable_change_authority` makes a self-issued exception look independent. Missing binding fields compare `null == null` and make `applicable` true. A missing or non-string `surface` gives an empty `r_eff`, so merge is allowed with no evidence. No registered fixture is malformed, so this has no consequence on the corpus, but the verifier must not fail open before it reads real records. Repair: `evaluate` first checks the fixture against the shape of contract §1. Every member is present with its type, `surface` is non-empty strings, entries and checks are objects with string ids, and the exception object carries every §1.3 member. A fixture that fails is refused: every predicate is `false`, `merge_allowed` is `false`, and the output names the first missing or mistyped member under `malformed`.
2. **A negative `cooling_off_hours` passes the ECC-Solo check (P3). Confirmed.** Repair: a negative value is malformed policy, and the fixture is refused as in item 1.
3. **`digest.mismatch` is never checked on repository records (P3). Confirmed.** Cited paths resolve beside the record, while repository records cite paths from the repository root, so the check was skipped silently. The change record's earlier "accepts 62" did not say so. Repair: when the file is not beside the record, the lint reads it from git at the record's own subject, `artifact_identity.revision`, and compares. A revision git does not have (the synthetic corpus) keeps the beside-the-record rule. That is the contract's rule, and no registered outcome changes. Observed: across the repository's 71 L0 evidence records, the old and new lint give identical verdicts (64 accepted). Every cited file that git has at the record's subject hashes to its cited digest. A copy of the P-B head record with one digest zeroed is accepted by the old lint and rejected by the new one with `digest.mismatch`. The regression test builds its record at `HEAD`, because CI's depth-2 checkout does not have older subjects.
4. **The differential test is weaker than described (P3). Confirmed.** Mutation arm 9 fell through to the producer arm, and no arm deletes a field. Repair: arm 9 is its own case. The reference is pinned by the registration, so the differential property still covers the contract's domain, well-formed fixtures. A new property deletes or retypes one member at a time and requires the verifier to refuse the result with `merge_allowed = false`. Where the reference also mishandles malformed input (`null == null`), that is out of its domain and recorded here, not repaired in a pinned artifact. One disagreement is inside the domain. Registered fixture V031 makes a negative `selected_tests` well formed, failing `passed`, so a fractional count is well formed too. The implementation does not pass it, because a test count is whole; the pinned reference passes it (`0.5 > 0`). The implementation's reading is the fail-closed one and is kept, with a test. Discriminating checks: with the refusal disabled, the new property and the review probes both fail; the registered corpus still decides 152/152.

Questions answered:
1. *Reference independence.* The reference was written by a session told not to read `tools/rha-verifier/src/`; `model.rs` was committed 6.5 minutes earlier. The claim is corrected to "without reading the implementation", not "before it existed".
2. *A missing or unknown `outcome` in lint evidence.* The contract has no code for it; it belongs to the `schema.*` cases of the schema-contract follow-on (decision `schema-contract`).
3. *`unknown` revisions, uppercase hex, and bare 64-hex digests.* These are the deliberate choices stated in `record_lint.rs`'s header. The historical records use them, and the schema contract tightens them per field.
4. *The entry's `kind`.* The verifier judges validity by the policy check's kind, as `Complete` and the contract specify; the kind-equality of `⊑` is carried by the join, which refuses mixed kinds.
5. *No L0 record at `522445f`.* That commit changed only `scope_globs`. The record for the repaired head follows these repairs.

### Repair attempts after the approving reviews (§11.7.10)

1. **`L0.clippy` failed on `7f9b4af`'s record (`evidence/CHG-019/20260923T001604Z-0a6b83370559.json`), and that head was pushed.** Hypothesis: the lint repair left a collapsible nested `if`. Discriminating check: `cargo clippy` reported `collapsible_if` at `record_lint.rs:164`, an error under the workspace lints. Change: the conditions are one `let` chain. Result: clippy is clean. The failed record is kept. The method lesson is the same as P-A's: run clippy before the commit, and never let a push follow a failed lane in one command chain.

### Not done in this packet, and why

- **JSON Schema files under `.rha/schemas/`** (W15). The generator found that the contract fixes no required fields, types or allowed keys, so the `schema.*` codes have no registered cases. Writing schemas now would be the invented contract §2.4 rule 6 forbids. The schema decision is taken under Kennedy's delegation of 2026-09-22 (task record decision `schema-contract`): required fields, types and allowed keys are derived mechanically from the record shapes committed so far and registered, with their malformed cases, before any schema file exists. That is its own follow-on item, not this packet.
- **Stage 4, keys and cut-over** (DP-4.1, DP-5.3): decided under Kennedy's delegation of 2026-09-22 (task record decisions `dp-4.1`, `dp-5.3`). The key cut-over is **not_run**: no producer isolated from the candidate exists, and a key the Executor or a candidate-run CI job could use would make `Authentic` vacuous. The bootstrap is not retired; its acceptance kind is renamed `bootstrap_acceptance` (proposal row 8) in `.rha/policy.toml`.

## Acceptance concerns

1. **`Authentic` cannot hold, by decision.** DP-4.1 creates no key (above). Every acceptance stays `bootstrap_acceptance` with `merge_allowed = false` until a producer isolated from the candidate exists. The spec's §11.7.6 model is implemented and graded at predicate level; the authentication channel stays in plan §10's deferred list.
2. **The schema files are a follow-on item** (above). Until they exist, Codex validates against `docs/architecture/verifier-contract.md` and the two corpus registrations (DP-5.4).
3. **This PR edits `.rha/policy.toml`, so `Applicable` is false for its records** (the candidate policy digest differs from the base; `evidence/CHG-019/20260922T233629Z-5a768afbe364.json`). That is the model working as specified: the edit is judged under the policy it replaces. Its acceptance is a controlled transition in the §11.5 sense, the first one this repository has had. The acceptance record says so and names the cause.
