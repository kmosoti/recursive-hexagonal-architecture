# P-B record-lint corpus registration

Fixed against `docs/architecture/verifier-contract.md` §2 before any lint or verifier implementation. `EXPECTED.json` records exact reason-code sets, not ordered diagnostics. No lint implementation was written or run.

Generation: `generation = "sampled"`; model `gpt-6-astra` (executor-selected generator session); sample indices R001–R080. No PRNG is used. The exact generation-prompt digest and source digests are recorded below.

**80 records: 20 accepted, 60 rejected.** Accepted: 6 task, 6 acceptance, 8 evidence. Total: 9 acceptance, 60 evidence, 11 task. Four digest-mismatch cases supply their cited file beside the record; two accepted controls also supply matching bytes.

## Reason counts

Counts are cases carrying each reason; one combined case carries four codes, so the sum exceeds 60.

| Reason | Count |
| --- | ---: |
| `schema.missing_field` | 0 |
| `schema.wrong_type` | 0 |
| `schema.unknown_field` | 0 |
| `evidence.duplicate_check_id` | 5 |
| `evidence.missing_required_check` | 5 |
| `evidence.passed_nonzero_exit` | 5 |
| `evidence.passed_invalid_kind` | 3 |
| `evidence.failed_without_error_class` | 4 |
| `evidence.not_run_without_reason` | 4 |
| `evidence.not_applicable_without_rule` | 3 |
| `evidence.inconclusive_outside_comparison` | 3 |
| `digest.malformed` | 11 |
| `revision.malformed` | 12 |
| `timestamp.malformed` | 4 |
| `digest.mismatch` | 4 |

## Shape and scope

- Task controls copy `.rha/tasks/CHG-019-trust.toml`; acceptance controls copy `.rha/acceptances/CHG-004.6.toml`. R001 and R007 are byte-for-byte copies. R002–R006 and R008–R012 vary only TOML presentation; their parsed values equal the respective source record exactly. The original complete shapes, including historical extension tables, are retained.
- Evidence uses `evidence/CHG-004.6/20260922T194644Z-20d42d90b69e.json`, including its **observed_checks** array, `selection_counts.tests`, nullable outcomes metadata and existing nested identities. Every evidence baseline keeps all eight policy L0 ids. Optional documentary provenance list entries with `sha256 = "unknown"`, and reviews carrying abbreviated documentary revisions, are removed; no missing digest or full revision is fabricated.
- These are parser/lint acceptances, not claims of authenticity or MergeAllowed. Archival controlled-transition acceptances and honest failed/not_run evidence are legitimate records. The lint contract has no disposition-consistency, live authority, tool-probe consistency, historical truth, or chronological-order reason code; semantic mutations preserve the envelope without asserting those additional validations.
- Existing policy_digest and base_policy_digest fields use prefixed SHA-256; lockfile_sha256 and source/artifact sha256 fields use bare lowercase hex, matching the source records. Date-only acceptor.date is not an RFC 3339 timestamp field. The timestamp cases target the existing *_at fields.
- Digest comparisons here are exercised against supplied fixture-local cited.txt bytes, as §2 specifies. Other archival paths are retained as source references; no missing-artifact rejection or lookup against the current checkout is invented. Recomputing historical references from Git, missing-file behavior, path sandboxing and remote references are not defined by this fixture interface.

## Omissions caused by the contract

- `schema.missing_field`: **0 cases**. The contract gives this reason code but never enumerates required fields or provides schemas. Existing records demonstrate presence and types, not requiredness. Deleting schema_version, an identity object or another candidate field would invent a required-field rule and potentially a cascading reason set. These cases are omitted pending an explicit schema contract.
- `schema.unknown_field`: **0 cases**. The contract restricts this diagnostic to top-level task/acceptance keys but never enumerates allowed keys. A key absent from example records could still be an allowed optional key. No allowed-key schema is invented.
- Optional versus required nested fields, unknown nested keys, evidence top-level unknown keys, unknown schema versions and enum values, invalid TOML/JSON syntax, nullability changes, and schema-error/semantic-error cascading are unspecified. No exact rejection set is assigned for them. `schema.wrong_type` also has **0 cases**: example scalar types are not a normative schema/type map. The planned string-version and numeric-id mutations were replaced with explicit malformed-revision witnesses before registration; no implementation result informed that decision.
- Per-field acceptance of bare versus prefixed digests, and uppercase bare hex, are not fixed explicitly. Negatives do not rely on rejecting an otherwise well-formed alternate encoding; they use wrong lengths, non-hex bytes or a disallowed prefix.
- The mapping of successful comparison/corpus validity metadata into the existing evidence envelope, and the names/locations of a valid not_applicable rule/rationale, are not supplied. Consequently no accepted comparison/corpus/not_applicable envelope is invented. Invalid not_applicable cases omit both rule and rationale, the explicit condition in the reason table. Test validity uses the established selection_counts.tests field.
- Missing error_class/reason means a present null field, or the explicitly empty reason case; no schema requiredness is inferred by deleting those keys. These are outcome-specific semantic defects named directly by the contract.
- Actor aliases, handoffs, graph cycles, revision existence, unsupported profiles and absent artifacts have no defined lint reason code here and are omitted.

## Case register

| ID | Kind | Exact reasons (empty means accept) | Construction |
| --- | --- | --- | --- |
| R001 | task | — | Unchanged CHG-019 task record |
| R002 | task | — | Leading comment preserves the real task |
| R003 | task | — | Equivalent literal-string title |
| R004 | task | — | Equivalent Unicode escape in the slug |
| R005 | task | — | Equivalent multiline-basic-string branch |
| R006 | task | — | Whitespace inside an empty optional array |
| R007 | acceptance | — | Unchanged CHG-004.6 controlled-transition acceptance |
| R008 | acceptance | — | Leading comment preserves the acceptance |
| R009 | acceptance | — | Equivalent TOML literal string |
| R010 | acceptance | — | Equivalent multiline-literal-string event |
| R011 | acceptance | — | Equivalent Unicode escapes in the date string |
| R012 | acceptance | — | Whitespace in outcomes array |
| R013 | evidence | — | Existing passing L0 envelope with documentary unknown digests and abbreviated review citations removed |
| R014 | evidence | — | Honest failed format check has nonzero status and an error class; lint acceptance is not eligibility |
| R015 | evidence | — | Honest not_run check has a reason and null exit status |
| R016 | evidence | — | Passed nextest selects two tests |
| R017 | evidence | — | L0 entry order is not check identity |
| R018 | evidence | — | Schema-valid ci class remains advisory; no protected evidence claim |
| R019 | evidence | — | Cited fixture digest matches supplied newline-terminated bytes (supplied: cited.txt) |
| R020 | evidence | — | Cited empty artifact has the SHA-256 of zero bytes (supplied: cited.txt) |
| R021 | evidence | digest.malformed | uppercase policy hex |
| R022 | evidence | digest.malformed | Prefixed policy digest contains non-hex characters |
| R023 | evidence | digest.malformed | policy digest has 63 digits |
| R024 | evidence | digest.malformed | wrong digest algorithm |
| R025 | evidence | digest.malformed | unknown is not a policy digest |
| R026 | evidence | digest.malformed | Bare lockfile digest contains an interior space |
| R027 | evidence | digest.malformed | bare lockfile digest has 63 digits |
| R028 | evidence | digest.malformed | bare lockfile digest has non-hex digits |
| R029 | evidence | digest.malformed | Bare lockfile digest contains non-hex symbols |
| R030 | evidence | digest.malformed | base policy digest has 65 digits |
| R031 | evidence | revision.malformed | artifact_identity.revision has 39 hex digits |
| R032 | evidence | revision.malformed | artifact_identity.revision has 41 hex digits |
| R033 | evidence | revision.malformed | Base revision has 40 non-hex characters |
| R034 | task | revision.malformed | Task base revision is a branch name |
| R035 | evidence | timestamp.malformed | started_at is not RFC 3339 with a valid offset: 2026-09-22T19:46:44.705 |
| R036 | evidence | timestamp.malformed | finished_at is not RFC 3339 with a valid offset: 2026-09-22 |
| R037 | evidence | timestamp.malformed | started_at is not RFC 3339 with a valid offset: 2026-09-22T19:46:44+25:00 |
| R038 | evidence | timestamp.malformed | A check start timestamp is malformed |
| R039 | evidence | evidence.duplicate_check_id | Duplicate existing check ids: L0.fmt |
| R040 | evidence | evidence.duplicate_check_id | Duplicate existing check ids: L0.typos |
| R041 | evidence | evidence.duplicate_check_id | Duplicate existing check ids: L0.nextest |
| R042 | evidence | evidence.duplicate_check_id | Duplicate existing check ids: L0.fmt, L0.typos |
| R043 | evidence | evidence.missing_required_check | Missing policy-required ids: L0.fmt |
| R044 | evidence | evidence.missing_required_check | Missing policy-required ids: L0.nextest |
| R045 | evidence | evidence.missing_required_check | Missing policy-required ids: L0.architecture |
| R046 | evidence | evidence.missing_required_check | Missing policy-required ids: L0.fmt, L0.typos |
| R047 | evidence | evidence.passed_nonzero_exit | L0.fmt says passed with exit_status=1 |
| R048 | evidence | evidence.passed_nonzero_exit | L0.nextest says passed with exit_status=2 |
| R049 | evidence | evidence.passed_nonzero_exit | L0.clippy says passed with exit_status=-1 |
| R050 | evidence | evidence.passed_nonzero_exit | L0.typos says passed with exit_status=127 |
| R051 | evidence | evidence.passed_invalid_kind | Passed test selected zero tests despite proptest_cases=256 |
| R052 | evidence | evidence.passed_invalid_kind | Passed test selected zero tests despite proptest_cases=512 |
| R053 | evidence | evidence.passed_invalid_kind | Passed test selected zero tests despite proptest_cases=4096 |
| R054 | evidence | evidence.failed_without_error_class | Failed without error class: L0.fmt |
| R055 | evidence | evidence.failed_without_error_class | Failed without error class: L0.clippy |
| R056 | evidence | evidence.failed_without_error_class | Failed without error class: L0.nextest |
| R057 | evidence | evidence.failed_without_error_class | Failed without error class: L0.fmt, L0.typos |
| R058 | evidence | evidence.not_run_without_reason | Invalid not_run: null reason and null exit |
| R059 | evidence | evidence.not_run_without_reason | Invalid not_run: reason present but exit status 0 |
| R060 | evidence | evidence.not_run_without_reason | Invalid not_run: empty reason and null exit |
| R061 | evidence | evidence.not_run_without_reason | Invalid not_run: reason present but nonzero exit status |
| R062 | evidence | evidence.not_applicable_without_rule | L0.fmt not_applicable with neither root rule id nor rationale |
| R063 | evidence | evidence.not_applicable_without_rule | L0.nextest not_applicable with neither root rule id nor rationale |
| R064 | evidence | evidence.not_applicable_without_rule | L0.clippy not_applicable with neither root rule id nor rationale |
| R065 | evidence | evidence.inconclusive_outside_comparison | Inconclusive outcome on kind format |
| R066 | evidence | evidence.inconclusive_outside_comparison | Inconclusive outcome on kind test |
| R067 | evidence | evidence.inconclusive_outside_comparison | Inconclusive outcome on kind lint |
| R068 | evidence | digest.mismatch | Supplied fixture bytes differ from the cited digest (supplied: cited.txt) |
| R069 | evidence | digest.mismatch | Supplied artifact bytes differ from the cited digest (supplied: cited.txt) |
| R070 | evidence | digest.mismatch | Supplied configuration bytes differ from the cited digest (supplied: cited.txt) |
| R071 | evidence | digest.mismatch | Supplied source bytes differ from the cited digest (supplied: cited.txt) |
| R072 | task | revision.malformed | Task base revision is an empty string |
| R073 | task | revision.malformed | Task base revision has an algorithm prefix instead of exactly 40 hex digits |
| R074 | acceptance | revision.malformed | Acceptance subject revision has 39 hex digits |
| R075 | acceptance | revision.malformed | Acceptance subject revision has 41 hex digits |
| R076 | task | revision.malformed | Task base revision has only 39 hex digits |
| R077 | acceptance | revision.malformed | Acceptance subject revision contains non-hex characters |
| R078 | evidence | revision.malformed | Revision has a leading space rather than exactly 40 hex characters |
| R079 | task | revision.malformed | Task base revision has 41 hex digits |
| R080 | evidence | digest.malformed, evidence.duplicate_check_id, evidence.missing_required_check, evidence.passed_nonzero_exit | Combined duplicate fmt, absent typos, passed/nonzero fmt and malformed lockfile digest |

## Generation inputs

Prompt digest: `sha256:d6152e73668ab704d8359e2ade6c2a0284ed2df93bf3e66a2234d2cad661b725` (UTF-8 bytes of the user generation request, beginning “You are the ADVERSARIAL CORPUS GENERATOR”, ending “Do not print file contents.”, LF newlines, no trailing newline; excludes preceding workspace instructions).

Model naming follows the supervising task record: `gpt-6-astra`; a runtime model attestation is not exposed. Sample indices are the fixture IDs. Source bytes at generation:

| Source | SHA-256 |
| --- | --- |
| `docs/architecture/verifier-contract.md` | `1feed322d85da895a91f046e3ffde4a6cb81246ccaf5adfd9b8796e157a140b2` |
| `docs/spec/rha-spec-v0.10.md` | `bd96216829035ade64c43befa5e672c58078f6afa4b04145c6183227e4e1071a` |
| `docs/plan/IMPLEMENTATION-PLAN.md` | `90fb3df3d73d94005e201d253ff6474203aeef200f54ca99c4ce1e69062c5d90` |
| `.rha/policy.toml` | `72bf2e5218b241b1dbf4ee4b96935fa89d45bd0a22fbe65c9c5a94b79bb314b3` |
| `.rha/tasks/CHG-019-trust.toml` | `92efd0aaee96990b15f0bfcb0c577e3d9ab597802955071a31327dd859b16570` |
| `.rha/acceptances/CHG-004.6.toml` | `3eb20267941bc0cbb03f4cd8b3da034c46d594348883fc49216994f0a56a5c40` |
| `evidence/CHG-004.6/20260922T194644Z-20d42d90b69e.json` | `76cfacd3ebe88a0816a54d3a3a9f8a45495f45bd6577f554d8e6e869bda6d428` |

Payload commitment: `sha256:4add0da428b9da2282bea05a6dd00df27c1294963af0e6d12734e5c5e796eda5`. Computed from 166 payload files, excluding REGISTRATION.md: sort relative POSIX paths lexically, serialize each as `<bare lowercase SHA-256>  <relative path>\n`, then SHA-256 the UTF-8 concatenation. This records the current registration proposal; the generator made no commit.

Validation at generation: every JSON/TOML payload parses; IDs are consecutive; expected fields and effective-obligation ordering are intact; the supplied file witnesses were hashed directly. There is no observed verifier/lint decision: neither implementation exists or ran, and Cargo was not invoked. Hand derivations and any advisory review are not a conformance result.

Independent advisory pre-registration review: `gpt-5.6-sol`, max reasoning, read-only. Reviewed all verifier derivation groups and record reason families against the fixed contract; no remaining concrete mistake or underdetermined written case was reported after the construction fixes. This was reasoning over fixtures, not an implementation run or corpus conformance grade.
