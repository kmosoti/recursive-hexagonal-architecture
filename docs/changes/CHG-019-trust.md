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

**Differential and model properties (§2.4 rule 4; W16).** A separate Codex session wrote an independent reference verifier, `tests/reference/mod.rs`, from the contract alone. It did so before this crate's code existed, and it was told not to read the corpus. The two implementations agree on all 152 fixtures and on 512 random mutations: surface, outcomes, test counts, duplicate entries, integrity, time, revocation, parameters, acceptor and producer. Property tests over the same mutations check Lemma 1 (obligations only grow), Lemma 3 (evidence cannot choose the obligations) and Proposition 2 (no silent pass). `rha-verifier corpus <dir> <out>` writes the H5 record to `evidence/h5/`.

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

### Review round 1 (Codex, `gpt-6-astra`, extra-high, read-only) on `2b5cd89`, determinations

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

### Not done in this packet, and why

- **JSON Schema files under `.rha/schemas/`** (W15). The generator found that the contract fixes no required fields, types or allowed keys, so the `schema.*` codes have no registered cases. Writing schemas now would be the invented contract §2.4 rule 6 forbids. They need a schema decision first, which is proposed as a gate for P-B's continuation.
- **Stage 4, keys and cut-over** (DP-4.1, DP-5.3): Kennedy's.

## Acceptance concerns

1. **DP-4.1:** trusted producer keys. Until they exist, `Authentic` is false for every record. The verifier computes it correctly, as 152 fixtures show, but no record can satisfy it.
2. **DP-5.3:** retiring `[acceptance.bootstrap]` is a policy edit that waits for stage 4.
3. **A schema contract** for `.rha/schemas/`, as above.
