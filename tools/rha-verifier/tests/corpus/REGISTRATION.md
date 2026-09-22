# P-B verifier corpus registration

Fixed against `docs/architecture/verifier-contract.md` §§1–1.3 before lint or verifier implementation. No implementation was run. Expectations are hand-assigned; JSON writing, sorting, and counts are mechanical.

Generation: `generation = "sampled"`; model `gpt-6-astra` (executor-selected generator session); sample indices V001–V152. No pseudorandom generator is used, so there is no PRNG seed. The user task is the generation prompt; its digest is recorded below.

Total: **152**. Eligible: **47**. Merge allowed: **57** (including **12** allowed solely by valid exception). Designated legitimate controls: **26**, all merge allowed. These are synthetic closed-world policy snapshots; their trust lists and waivable L1 checks do not change the repository policy. All eight actual L0 IDs stay non-waivable whenever included as waiver targets.

| Row | Count |
| --- | ---: |
| 11.7.9#1 | 16 |
| 11.7.9#2 | 8 |
| 11.7.9#3 | 20 |
| 11.7.9#4 | 12 |
| 11.7.9#6 | 8 |
| 11.7.9#7 | 12 |
| 11.7.9#8 | 18 |
| 11.7.9#9 | 10 |
| 11.7.9#11 | 12 |
| 17.2 | 10 |
| legitimate | 26 |

## Coverage and omitted cases

- Table rows #1, #2, #3, #4, #6, #7, #8, #9 and #11 have at least three cases each. Row #4 fixture-set mismatch is represented by the contract's corpus validity predicate: Applicable stays true and Passed fails. It does not silently extend the envelope identity rule.
- Row #5 and the §17.2 nested-guide network/secret attack are omitted: no permissions, grants, untrusted text or execution interface exists in the fixture contract. Row #10 (harness discovery), #12 (impact graph), #13 (work cycle/handoff), and #14 (team integration) lack harness/graph/team inputs. Ordinary wrong-integration-tree binding is covered by #4, without claiming team completion coverage.
- §17.2 graph deletion, stale model, missing/duplicate graph node, dropped-cycle edge and premature downstream task cannot be encoded. Missing/duplicate **check** ids, protected edits, wrong/expired exceptions, zero tests and inconclusive outcomes are covered.
- Raw confidence intervals and a benchmark falsely relabelled passed with performed=true cannot be graded: the contract provides no interval or recomputation inputs. Inconclusive and performed=false are tested; interval truth is not invented.
- Renamed-accountable-human equivalence cannot be inferred: no alias/accountability graph is provided. The renamed unlisted issuer is rejected by explicit authority membership only.
- Deletion/disable directives, conflicting check kinds, undefined required ids, malformed fixture inputs, null or non-string required-input values, root not-applicable rationale, cryptographic verification, revocation-log lookup, and append-only-log tampering have no specified input/output semantics. No expectations are invented for them.
- Conflicting-outcome duplicate required entries leave the singular entry used by Passed unspecified. Duplicate extras test the global duplicate prohibition with an unambiguous required entry. Missing required entries are paired with a retained required failure so Passed is unambiguously false.
- For comparable local weakening, contract §1.1 keeps the stricter root value without a conflict; only incomparable parameters set conflict=true. A literal reading of table row #1 that flags every weaker value would contradict the fixed contract, so it is not used.

## Hand derivations

Each JSON contains the complete sorted `r_eff` and every graded predicate. Below, A=authentic, I=applicable, C=complete, P=passed, E=eligible, M=merge_allowed; 1=true and 0=false. X is valid_exception (`—` means null). No short-circuiting is assumed between independent predicates. Conflict cases have null obligations and all downstream predicates false.

| ID | Row | A I C P E M | X | Construction and derivation |
| --- | --- | --- | --- | --- |
| V001 | 11.7.9#1 | 1 1 1 1 1 1 | — | Local delta loosening retains the root value. delta: root 0.03 dominates local 0.1; entry satisfies the retained root. |
| V002 | 11.7.9#1 | 1 1 1 1 1 1 | — | Local n loosening retains the root value. n: root 30 dominates local 10; entry satisfies the retained root. |
| V003 | 11.7.9#1 | 1 1 1 1 1 1 | — | Local executions loosening retains the root value. executions: root 5 dominates local 1; entry satisfies the retained root. |
| V004 | 11.7.9#1 | 1 1 1 1 1 1 | — | Local proptest_cases loosening retains the root value. proptest_cases: root 256 dominates local 32; entry satisfies the retained root. |
| V005 | 11.7.9#1 | 1 1 1 1 1 1 | — | Local selection loosening retains the root value. selection: root ['unit', 'integration'] dominates local ['unit']; entry satisfies the retained root. |
| V006 | 11.7.9#1 | 1 1 1 1 1 1 | — | An empty local check list cannot disable nextest. The root rule still requires nextest; the local policy adds nothing. |
| V007 | 11.7.9#1 | 0 0 0 0 0 0 | — | Unordered timeout 600 versus 300 conflicts. Unequal unordered timeouts make the join undefined; every downstream predicate is false. |
| V008 | 11.7.9#1 | 0 0 0 0 0 0 | — | Unordered timeout 600 versus 1200 conflicts. Unequal unordered timeouts make the join undefined; every downstream predicate is false. |
| V009 | 11.7.9#1 | 0 0 0 0 0 0 | — | Incomparable selections do not silently union. Neither ['unit'] nor ['integration'] contains the other. |
| V010 | 11.7.9#1 | 0 0 0 0 0 0 | — | Incomparable selections do not silently union. Neither ['unit', 'smoke'] nor ['unit', 'integration'] contains the other. |
| V011 | 11.7.9#1 | 1 1 1 1 1 1 | — | A local superset strengthens the root selection. The local selection contains the root selection; the matching stronger entry passes. |
| V012 | 11.7.9#1 | 1 1 1 1 1 1 | — | A parameter present only locally is retained. n=50 joins into the root; evidence includes it. |
| V013 | 11.7.9#1 | 0 0 0 0 0 0 | — | Different opaque parameters conflict. engine has equality-only ordering; alpha and beta are incomparable. |
| V014 | 11.7.9#1 | 1 1 1 1 1 1 | — | An identical local check is an idempotent join. Every shared parameter is equal. |
| V015 | 11.7.9#1 | 1 1 1 1 1 1 | — | A triggered local rule adds a comparison. Both obligations remain and have valid unique entries. |
| V016 | 11.7.9#1 | 1 1 0 0 0 0 | — | Ignoring an added local obligation cannot yield eligibility. The comparison is missing (Complete=false); retained nextest fails (Passed=false). |
| V017 | 11.7.9#2 | 1 1 1 1 1 1 | — | Protected edit retains obligations: tools/rha-verifier/src/main.rs. The root protected rule matches this exact path; empty local policy removes nothing. |
| V018 | 11.7.9#2 | 1 1 1 1 1 1 | — | Protected edit retains obligations: .rha/policy.toml. The root protected rule matches this exact path; empty local policy removes nothing. |
| V019 | 11.7.9#2 | 1 1 1 1 1 1 | — | Protected edit retains obligations: crates/core/AGENTS.md. The root protected rule matches this exact path; empty local policy removes nothing. |
| V020 | 11.7.9#2 | 1 1 1 1 1 1 | — | Protected edit retains obligations: xtask/src/main.rs. The root protected rule matches this exact path; empty local policy removes nothing. |
| V021 | 11.7.9#2 | 1 0 1 1 0 0 | — | Candidate policy digest cannot authorize tools/rha-verifier/src/main.rs. Protected obligations and valid entries remain; only the evidence policy binding fails. |
| V022 | 11.7.9#2 | 1 0 1 1 0 0 | — | Candidate policy digest cannot authorize .rha/policy.toml. Protected obligations and valid entries remain; only the evidence policy binding fails. |
| V023 | 11.7.9#2 | 1 0 1 1 0 0 | — | Candidate policy digest cannot authorize AGENTS.md. Protected obligations and valid entries remain; only the evidence policy binding fails. |
| V024 | 11.7.9#2 | 1 1 1 0 0 0 | — | Protected edit with a failed root test is blocked. The policy edit triggers both root checks; nextest remains failed. |
| V025 | 11.7.9#3 | 1 1 0 0 0 0 | — | Missing L0.nextest plus a retained failure. A required entry is absent; the other required check explicitly fails, fixing Passed=false independently. |
| V026 | 11.7.9#3 | 1 1 0 0 0 0 | — | Missing L0.fmt plus a retained failure. A required entry is absent; the other required check explicitly fails, fixing Passed=false independently. |
| V027 | 11.7.9#3 | 1 1 0 1 0 0 | — | Duplicate non-required IDs with passed outcomes. No entry id may repeat, including extra ids; the sole required entry still passes. |
| V028 | 11.7.9#3 | 1 1 0 1 0 0 | — | Duplicate non-required IDs with failed outcomes. No entry id may repeat, including extra ids; the sole required entry still passes. |
| V029 | 11.7.9#3 | 1 1 0 1 0 0 | — | Duplicate non-required IDs with not_run outcomes. No entry id may repeat, including extra ids; the sole required entry still passes. |
| V030 | 11.7.9#3 | 1 1 1 0 0 0 | — | Passed test has selected_tests=0. Entry parameters cover the root, but selected_tests is not greater than zero. |
| V031 | 11.7.9#3 | 1 1 1 0 0 0 | — | Passed test has selected_tests=-1. Entry parameters cover the root, but selected_tests is not greater than zero. |
| V032 | 11.7.9#3 | 1 1 1 0 0 0 | — | Passed test has selected_tests=0. Entry parameters cover the root, but selected_tests is not greater than zero. |
| V033 | 11.7.9#3 | 1 1 0 1 0 0 | — | Weaker entry parameter n. Entry n=29 does not refine 30; positive test count still makes Passed=true. |
| V034 | 11.7.9#3 | 1 1 0 1 0 0 | — | Weaker entry parameter executions. Entry executions=4 does not refine 5; positive test count still makes Passed=true. |
| V035 | 11.7.9#3 | 1 1 0 1 0 0 | — | Weaker entry parameter proptest_cases. Entry proptest_cases=255 does not refine 256; positive test count still makes Passed=true. |
| V036 | 11.7.9#3 | 1 1 0 1 0 0 | — | Weaker entry parameter delta. Entry delta=0.04 does not refine 0.03; positive test count still makes Passed=true. |
| V037 | 11.7.9#3 | 1 1 0 1 0 0 | — | Weaker entry parameter selection. Entry selection=['unit'] does not refine ['unit', 'integration']; positive test count still makes Passed=true. |
| V038 | 11.7.9#3 | 1 1 0 1 0 0 | — | Entry omits a required parameter. timeout_secs=600 is not supplied; the test outcome and validity still pass. |
| V039 | 11.7.9#3 | 1 1 0 1 0 0 | — | Entry changes an unordered timeout. Different unordered timeout cannot refine 600; Passed is independent of parameter strength. |
| V040 | 11.7.9#3 | 1 1 1 0 0 0 | — | A required test outcome is failed. The unique entry has sufficient params but outcome is not passed. |
| V041 | 11.7.9#3 | 1 1 1 0 0 0 | — | A required test outcome is not_run. The unique entry has sufficient params but outcome is not passed. |
| V042 | 11.7.9#3 | 1 1 1 0 0 0 | — | A required test outcome is not_applicable. The unique entry has sufficient params but outcome is not passed. |
| V043 | 11.7.9#3 | 1 1 1 0 0 0 | — | A required test outcome is inconclusive. The unique entry has sufficient params but outcome is not passed. |
| V044 | 11.7.9#3 | 1 1 1 0 0 0 | — | A passed comparison was not performed. Comparison kind validity requires performed=true. |
| V045 | 11.7.9#4 | 1 0 1 1 0 0 | — | Evidence subject binding differs (3). Only the equality for evidence.subject fails; all entry predicates remain true. |
| V046 | 11.7.9#4 | 1 0 1 1 0 0 | — | Evidence subject binding differs (4). Only the equality for evidence.subject fails; all entry predicates remain true. |
| V047 | 11.7.9#4 | 1 0 1 1 0 0 | — | Evidence policy binding differs (3). Only the equality for evidence.policy fails; all entry predicates remain true. |
| V048 | 11.7.9#4 | 1 0 1 1 0 0 | — | Evidence policy binding differs (4). Only the equality for evidence.policy fails; all entry predicates remain true. |
| V049 | 11.7.9#4 | 1 0 1 1 0 0 | — | Evidence base binding differs (3). Only the equality for evidence.base fails; all entry predicates remain true. |
| V050 | 11.7.9#4 | 1 0 1 1 0 0 | — | Evidence base binding differs (4). Only the equality for evidence.base fails; all entry predicates remain true. |
| V051 | 11.7.9#4 | 1 1 1 0 0 0 | — | Evidence names a different fixture-set digest. Envelope identity is applicable; corpus kind validity fails on the digest, so Passed=false. |
| V052 | 11.7.9#4 | 1 1 1 0 0 0 | — | Evidence names a different fixture-set digest. Envelope identity is applicable; corpus kind validity fails on the digest, so Passed=false. |
| V053 | 11.7.9#4 | 1 1 0 0 0 0 | — | Both entry params and actual corpus cite another set. Unequal equality-only corpus parameter fails Complete; the actual digest differs from both required and entry corpus params, unambiguously failing Passed. |
| V054 | 11.7.9#4 | 1 0 1 1 0 0 | — | Fresh integration tree invalidates prior passing evidence. The evidence remains bound to tree 222… rather than the new candidate 555…. |
| V055 | 11.7.9#4 | 1 0 1 1 0 0 | — | Moving the protected base invalidates old evidence. The evidence base remains 111…; current base is 666…. |
| V056 | 11.7.9#4 | 1 0 1 1 0 0 | — | All three evidence bindings are stale. All binding equalities fail; authenticity, completeness and passed remain independently true. |
| V057 | 11.7.9#6 | 1 1 1 0 0 0 | — | Inconclusive comparison with performed=True and n=30. No outcome labelled inconclusive is passed, regardless of performance flag or stronger sample count. |
| V058 | 11.7.9#6 | 1 1 1 0 0 0 | — | Inconclusive comparison with performed=False and n=30. No outcome labelled inconclusive is passed, regardless of performance flag or stronger sample count. |
| V059 | 11.7.9#6 | 1 1 1 0 0 0 | — | Inconclusive comparison with performed=True and n=60. No outcome labelled inconclusive is passed, regardless of performance flag or stronger sample count. |
| V060 | 11.7.9#6 | 1 1 1 0 0 0 | — | Relabelled passed without performing the comparison. The passed label cannot bypass performed=false. |
| V061 | 11.7.9#6 | 1 1 1 0 0 0 | — | Performed comparison explicitly failed. Performed=true establishes validity, not a passing outcome. |
| V062 | 11.7.9#6 | 1 1 1 1 1 1 | — | Performed and passed comparison is eligible. The comparison ran, passed and uses the exact approved params. |
| V063 | 11.7.9#6 | 1 1 1 0 0 1 | true | A valid exception can cover an inconclusive waivable comparison. Passed stays false; independent authorized exception covers the sole failed check and allows merge. |
| V064 | 11.7.9#6 | 1 1 1 0 0 0 | false | An exception that omits the inconclusive check is invalid. An inconclusive outcome is among the failed checks that waived must cover. |
| V065 | 11.7.9#7 | 1 1 1 0 0 0 | false | ECC-Team rejects self-issued exception. Issuer equals accountable authority; ECC-Team has no Solo exception to independence. |
| V066 | 11.7.9#7 | 1 1 1 0 0 0 | false | Old log does not authorize self-issuance under ECC-Team. Even a long elapsed delay cannot apply the ECC-Solo-only choice to ECC-Team. |
| V067 | 11.7.9#7 | 1 1 1 0 0 0 | false | ECC-Solo one second inside cooling-off. Only 23h59m59s have elapsed since logging, less than 24 hours. |
| V068 | 11.7.9#7 | 1 1 1 0 0 1 | true | ECC-Solo exactly at cooling-off boundary. now equals logged_at + 24h, satisfying the inclusive cooling boundary. |
| V069 | 11.7.9#7 | 1 1 1 0 0 1 | true | ECC-Solo one second after cooling-off. 24h00m01s have elapsed, with all other exception clauses satisfied. |
| V070 | 11.7.9#7 | 1 1 1 0 0 0 | false | ECC-Solo self-issuance without a log timestamp. The self-issued Solo path explicitly requires logged_at to be present. |
| V071 | 11.7.9#7 | 1 1 1 0 0 0 | false | ECC-Solo log timestamp lies in the future. The future log cannot have completed cooling-off. |
| V072 | 11.7.9#7 | 1 1 1 0 0 0 | false | Waiving an unrelated non-waivable check invalidates exception. waived intersects non_waivable even though fmt is not an effective obligation here. |
| V073 | 11.7.9#7 | 1 1 1 0 0 0 | false | An actually failed non-waivable test cannot be excepted. waived covers the failure but intersects the non-waivable nextest check. |
| V074 | 11.7.9#7 | 1 1 1 0 0 0 | false | Renaming an agent does not put it in exception authority. agent:renamed-reviewer is absent from the explicit authority list; no alias resolution is assumed. |
| V075 | 11.7.9#7 | 1 1 1 0 0 1 | true | Independent ECC-Team exception is valid. human:reviewer is authorized and differs from the accountable human; every other clause holds. |
| V076 | 11.7.9#7 | 1 1 1 0 0 0 | false | Independent issuer still needs explicit authority. Independence alone does not make human:reviewer an authorized issuer. |
| V077 | 11.7.9#8 | 1 1 1 0 0 0 | false | Invalid exception expires_at = 2026-09-22T11:59:59Z. Expired one second ago. Evidence remains authentic, applicable, complete, and failed. |
| V078 | 11.7.9#8 | 1 1 1 0 0 0 | false | Invalid exception expires_at = 2026-09-22T12:00:00Z. Expiry is exclusive: now equals expires_at. Evidence remains authentic, applicable, complete, and failed. |
| V079 | 11.7.9#8 | 1 1 1 0 0 0 | false | Invalid exception issued_at = 2026-09-22T12:00:01Z. Issuance is one second in the future. Evidence remains authentic, applicable, complete, and failed. |
| V080 | 11.7.9#8 | 1 1 1 0 0 0 | false | Invalid exception revoked = True. Revocation defeats an otherwise valid exception. Evidence remains authentic, applicable, complete, and failed. |
| V081 | 11.7.9#8 | 1 1 1 0 0 0 | false | Invalid exception subject = 3333333333333333333333333333333333333333. Exception subject differs from candidate_tree. Evidence remains authentic, applicable, complete, and failed. |
| V082 | 11.7.9#8 | 1 1 1 0 0 0 | false | Invalid exception base = 3333333333333333333333333333333333333333. Exception base differs from protected base. Evidence remains authentic, applicable, complete, and failed. |
| V083 | 11.7.9#8 | 1 1 1 0 0 0 | false | Invalid exception policy = sha256:3333333333333333333333333333333333333333333333333333333333333333. Exception policy differs from root digest. Evidence remains authentic, applicable, complete, and failed. |
| V084 | 11.7.9#8 | 1 1 1 0 0 0 | false | Invalid exception authentic = False. Exception authentication fails. Evidence remains authentic, applicable, complete, and failed. |
| V085 | 11.7.9#8 | 1 1 1 0 0 0 | false | Invalid exception reason = . The required reason is empty. Evidence remains authentic, applicable, complete, and failed. |
| V086 | 11.7.9#8 | 1 1 1 0 0 0 | false | Invalid exception compensating_control = . The compensating control is empty. Evidence remains authentic, applicable, complete, and failed. |
| V087 | 11.7.9#8 | 1 1 1 0 0 0 | false | Invalid exception follow_up = . The follow-up is empty. Evidence remains authentic, applicable, complete, and failed. |
| V088 | 11.7.9#8 | 1 1 1 0 0 0 | false | Exception does not cover the failed comparison. The sole failed check is missing from waived. |
| V089 | 11.7.9#8 | 0 1 1 0 0 0 | false | Valid exception cannot rescue untrusted evidence. Authentic evidence is a prerequisite to ValidException. |
| V090 | 11.7.9#8 | 1 0 1 0 0 0 | false | Valid exception cannot rescue inapplicable evidence. Applicable evidence is a prerequisite even when the exception itself binds correctly. |
| V091 | 11.7.9#8 | 1 1 0 0 0 0 | false | Valid exception cannot rescue incomplete evidence. Weaker n fails Complete; a valid exception requires Complete. |
| V092 | 11.7.9#8 | 1 1 1 0 0 1 | true | Exception issued exactly now is valid. issued_at <= now is inclusive; the issuer is independent and expiry is tomorrow. |
| V093 | 11.7.9#8 | 1 1 1 0 0 1 | true | Exception remains valid one second before expiry. now is strictly before expiry and all other clauses hold. |
| V094 | 11.7.9#8 | 1 1 1 1 1 1 | false | Invalid optional exception does not invalidate eligible evidence. The empty reason makes ValidException=false; Eligible=true independently allows the authorized merge. |
| V095 | 11.7.9#9 | 1 1 1 1 1 1 | — | Unmatched path receives defaults: README.md. No root scope matches; default fmt is required and passed. |
| V096 | 11.7.9#9 | 1 1 1 1 1 1 | — | Unmatched path receives defaults: new/hidden.rs. No root scope matches; default fmt is required and passed. |
| V097 | 11.7.9#9 | 1 1 1 1 1 1 | — | Unmatched path receives defaults: crates/other/src/lib.rs. No root scope matches; default fmt is required and passed. |
| V098 | 11.7.9#9 | 1 1 1 1 1 1 | — | Unmatched path receives defaults: crates/corex/src/lib.rs. No root scope matches; default fmt is required and passed. |
| V099 | 11.7.9#9 | 1 1 1 1 1 1 | — | One unmatched path adds defaults to matched obligations. core triggers nextest; README triggers default fmt; both unique entries pass. |
| V100 | 11.7.9#9 | 1 1 0 0 0 0 | — | Matched path cannot hide a missing default obligation. Unmatched README requires absent fmt; nextest explicitly fails, fixing Passed=false. |
| V101 | 11.7.9#9 | 1 1 1 1 1 1 | — | Multiple unmatched paths deduplicate the default obligation. Both paths add the same fmt id, yielding one required check and one entry. |
| V102 | 11.7.9#9 | 1 1 1 1 1 1 | — | Single star cannot cross a slash. docs/*.md does not match docs/a/b.md; default fmt applies. |
| V103 | 11.7.9#9 | 1 1 1 1 1 1 | — | Double star matches zero directory segments. docs/**/readme.md matches docs/readme.md; nextest is triggered and defaults are not. |
| V104 | 11.7.9#9 | 1 1 1 1 1 1 | — | Double star matches several directory segments. The ** consumes a/b; the triggered root test passes. |
| V105 | 11.7.9#11 | 0 1 1 1 0 0 | — | Untrusted producer agent:executor. Producer is absent from the exact trusted_producers list; all remaining evidence predicates hold. |
| V106 | 11.7.9#11 | 0 1 1 1 0 0 | — | Untrusted producer automation:CI. Producer is absent from the exact trusted_producers list; all remaining evidence predicates hold. |
| V107 | 11.7.9#11 | 0 1 1 1 0 0 | — | Untrusted producer human:kennedy. Producer is absent from the exact trusted_producers list; all remaining evidence predicates hold. |
| V108 | 11.7.9#11 | 0 1 1 1 0 0 | — | Trusted producer with invalid integrity. Authentic requires integrity=valid even for a trusted producer. |
| V109 | 11.7.9#11 | 0 1 1 1 0 0 | — | Trusted producer with absent integrity. Authentic requires integrity=valid even for a trusted producer. |
| V110 | 11.7.9#11 | 0 1 1 1 0 0 | — | Bootstrap empty trust list denies every producer. No producer is in the empty trust set. |
| V111 | 11.7.9#11 | 1 0 1 1 0 0 | — | Required input toolchain is unknown. Every required input must differ from unknown; check results cannot repair applicability. |
| V112 | 11.7.9#11 | 1 0 1 1 0 0 | — | Required input lockfile is unknown. Every required input must differ from unknown; check results cannot repair applicability. |
| V113 | 11.7.9#11 | 1 0 1 1 0 0 | — | Required input is empty. The contract excludes empty required input values. |
| V114 | 11.7.9#11 | 1 0 1 1 0 0 | — | Required input is absent. lockfile is required but missing. |
| V115 | 11.7.9#11 | 0 1 1 1 0 0 | — | Untrusted producer and invalid integrity both fail authenticity. Both Authentic clauses fail, while the independent remaining predicates still hold. |
| V116 | 11.7.9#11 | 0 0 1 1 0 0 | — | Missing integrity and unknown artifact identity combine. Authentic=false and Applicable=false; entries remain complete and passed. |
| V117 | 17.2 | 1 1 1 1 1 0 | — | Eligible evidence cannot authorize an outside acceptor. Eligible=true; acceptor is absent from acceptance_authority, so merge is denied. |
| V118 | 17.2 | 1 1 1 0 0 0 | true | Valid exception cannot authorize an outside acceptor. ValidException=true, but the separate acceptance authority condition fails. |
| V119 | 17.2 | 1 1 1 1 1 0 | — | Empty acceptance authority denies an otherwise eligible merge. No acceptor can belong to the empty authority set. |
| V120 | 17.2 | 1 1 1 0 0 0 | — | Trigger evasion still incurs a failed default check. The unmatched policy path receives fmt; its failed outcome blocks eligibility. |
| V121 | 17.2 | 1 1 1 0 0 0 | — | Passing tests do not conceal an inconclusive comparison. Both obligations are complete, but the comparison prevents all-check Passed. |
| V122 | 17.2 | 1 1 1 0 0 0 | — | Passing comparison does not conceal zero selected tests. The test kind criterion fails even though the other required check passes. |
| V123 | 17.2 | 1 1 0 1 0 0 | false | An exception cannot repair duplicate extra check IDs. Duplicate fmt ids fail Complete even though fmt is not required; exceptions require completeness. |
| V124 | 17.2 | 1 0 1 0 0 0 | false | Matching candidate-selected evidence and exception policies still fail. Both documents agree with each other but neither binds to the protected root digest. |
| V125 | 17.2 | 1 1 1 0 0 0 | false | Wrong-subject and expired exception remains invalid. Both subject binding and acceptance-time expiry checks fail. |
| V126 | 17.2 | 0 0 0 0 0 0 | false | An exception cannot rescue a join conflict. Unordered timeout conflict sets r_eff=null and all downstream predicates, including the supplied exception, false. |
| V127 | legitimate | 1 1 1 1 1 1 | — | Minimal ordinary contribution. Exact unique test evidence with one selected test and authorized acceptance. |
| V128 | legitimate | 1 1 1 1 1 1 | — | Format-only contribution. Format has no additional kind criterion; its passed entry suffices. |
| V129 | legitimate | 1 1 1 1 1 1 | — | Performed comparison at exact policy parameters. performed=true and outcome=passed satisfy the comparison validity rule. |
| V130 | legitimate | 1 1 1 1 1 1 | — | Corpus check uses exactly the required digest. Actual and required corpus digests are equal. |
| V131 | legitimate | 1 1 1 1 1 1 | — | Several kinds pass together with sorted effective obligations. All four obligations have unique exact entries and satisfy their kind criteria; r_eff is lexically sorted. |
| V132 | legitimate | 1 1 1 1 1 1 | — | Evidence may select a strict superset. The entry selection contains unit and still selects a positive number of tests. |
| V133 | legitimate | 1 1 1 1 1 1 | — | Stronger evidence parameter n. 31 is at least as strict as 30 in the declared n ordering. |
| V134 | legitimate | 1 1 1 1 1 1 | — | Stronger evidence parameter executions. 6 is at least as strict as 5 in the declared executions ordering. |
| V135 | legitimate | 1 1 1 1 1 1 | — | Stronger evidence parameter proptest_cases. 512 is at least as strict as 256 in the declared proptest_cases ordering. |
| V136 | legitimate | 1 1 1 1 1 1 | — | Stronger evidence parameter delta. 0.02 is at least as strict as 0.03 in the declared delta ordering. |
| V137 | legitimate | 1 1 1 1 1 1 | — | Extra entry parameter does not weaken a requirement. Every required parameter is still covered; the extra engine key adds information. |
| V138 | legitimate | 1 1 1 1 1 1 | — | A unique failed extra check does not fail required checks. Passed quantifies over r_eff; the unique extra id neither duplicates nor replaces nextest. |
| V139 | legitimate | 1 1 1 1 1 1 | — | Unknown non-required input is permitted. Only toolchain and lockfile are required; both remain known. |
| V140 | legitimate | 1 1 1 1 1 1 | — | A second explicitly trusted producer is accepted. The exact producer id is in the declared trust set and integrity is valid. |
| V141 | legitimate | 1 1 1 1 1 1 | — | A second explicit acceptance authority can merge. Eligible evidence and explicit acceptor membership both hold. |
| V142 | legitimate | 1 1 1 1 1 1 | — | ECC-Team normal eligibility needs no exception. The independent-issuer rule is irrelevant when no exception exists. |
| V143 | legitimate | 1 1 1 1 1 1 | — | Overlapping root rules retain one check per ID. Both rules require the same check, whose identical join is idempotent. |
| V144 | legitimate | 1 1 1 1 1 1 | — | Non-applicable local policy cannot cause a conflict. No surface path matches the local scope; only the root check participates. |
| V145 | legitimate | 1 1 1 1 1 1 | — | Several matched paths do not multiply check entries. The same triggered obligation is required once. |
| V146 | legitimate | 1 1 1 0 0 1 | true | Independent valid exception permits a failed waivable comparison. Every exception clause holds; the original failed outcome remains failed. |
| V147 | legitimate | 1 1 1 0 0 1 | true | Self-issued ECC-Solo exception after 48 hours. The authentic self-issued exception was logged 48 hours ago, exceeding the 24-hour delay. |
| V148 | legitimate | 1 1 1 0 0 1 | true | Independent ECC-Team issuer does not require a Solo log. Independent issuance satisfies the main clause; logged_at is required only on the Solo self-issued path. |
| V149 | legitimate | 1 1 1 0 0 1 | true | One valid exception covers two waivable failed checks. Both failed ids are covered, and neither is non-waivable; all identity and authority clauses hold. |
| V150 | legitimate | 1 1 1 0 0 1 | true | Passed non-waivable test plus valid comparison exception. Only the failed comparison is waived; the non-waivable test passes normally. |
| V151 | legitimate | 1 1 1 1 1 1 | true | Eligible evidence with a valid empty waiver. The set of failed checks is empty, so empty waived covers it; all remaining exception clauses hold. |
| V152 | legitimate | 1 1 1 0 0 1 | true | Declared zero-hour Solo cooling at issuance instant. The fixture explicitly declares zero cooling; issued_at and logged_at equal now, satisfying both inclusive boundaries. |

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

Payload commitment: `sha256:17ae1468e8c1d3d36696ce75148b6854df7f5dc0d0a5581768ba63c6736c52da`. Computed from 152 payload files, excluding REGISTRATION.md: sort relative POSIX paths lexically, serialize each as `<bare lowercase SHA-256>  <relative path>\n`, then SHA-256 the UTF-8 concatenation. This records the current registration proposal; the generator made no commit.

Validation at generation: every JSON/TOML payload parses; IDs are consecutive; expected fields and effective-obligation ordering are intact; the supplied file witnesses were hashed directly. There is no observed verifier/lint decision: neither implementation exists or ran, and Cargo was not invoked. Hand derivations and any advisory review are not a conformance result.

Independent advisory pre-registration review: `gpt-5.6-sol`, max reasoning, read-only. Reviewed all verifier derivation groups and record reason families against the fixed contract; no remaining concrete mistake or underdetermined written case was reported after the construction fixes. This was reasoning over fixtures, not an implementation run or corpus conformance grade.
