# Verifier and record-lint contract (P-B, fixed before any code)

This is the interface that P-B's registered corpora grade. It was written in P-B stage 1, before any lint or verifier code existed, so that a separate generator session can write fixtures with unambiguous expectations (plan §2.4 rule 2). It restates spec §11.7.6 in executable terms and does not extend it. Where the spec leaves a choice open, the choice is marked **(choice)** and applies to every fixture alike.

## 1. Verifier fixtures

A fixture is one JSON file: `tools/rha-verifier/tests/corpus/<id>.json`. The verifier reads the fixture without its `expected` member and must produce `expected` exactly. Every key of `expected` is graded.

```json
{
  "id": "V001",
  "row": "11.7.9#1 | 17.2 | legitimate",
  "purpose": "one line",
  "policy": {
    "digest": "sha256:<64 hex>",
    "profile": "ECC-Solo | ECC-Team",
    "checks": [{"id": "L0.nextest", "kind": "test", "params": {"selection": ["a", "b"], "timeout_secs": 600}}],
    "rules": [{"id": "core.change", "scope": "crates/core/**", "requires": ["L0.nextest"]}],
    "default_obligations": ["L0.fmt"],
    "non_waivable": ["L0.fmt"],
    "trusted_producers": ["automation:ci"],
    "required_inputs": ["toolchain", "lockfile"],
    "exception_authority": ["human:reviewer"],
    "acceptance_authority": ["human:kennedy"],
    "cooling_off_hours": 24
  },
  "local_policies": [{"scope": "crates/core/**", "checks": [], "rules": []}],
  "base": "<40 hex>",
  "candidate_tree": "<40 hex>",
  "surface": ["crates/core/src/lib.rs"],
  "evidence": {
    "producer": "automation:ci",
    "integrity": "valid | invalid | absent",
    "subject": "<40 hex tree>",
    "policy": "sha256:<64 hex>",
    "base": "<40 hex>",
    "inputs": {"toolchain": "1.98.1", "lockfile": "sha256:<64 hex>"},
    "entries": [{"id": "L0.nextest", "kind": "test", "params": {"selection": ["a", "b"]}, "outcome": "passed", "selected_tests": 12}]
  },
  "exception": null,
  "acceptor": "human:kennedy",
  "accountable_change_authority": "human:kennedy",
  "now": "2026-09-22T12:00:00Z",
  "expected": {
    "r_eff": [{"id": "L0.nextest", "kind": "test", "params": {"selection": ["a", "b"], "timeout_secs": 600}}],
    "conflict": false,
    "authentic": true, "applicable": true, "complete": true, "passed": true,
    "eligible": true, "valid_exception": null, "merge_allowed": true
  }
}
```

### 1.1 Obligations

- **Triggers.** A root rule triggers when any `surface` path matches its `scope` glob (`**` is any number of segments, `*` is characters within one segment). **Total classification:** if some path matches no root rule, `default_obligations` also apply.
- **Local policies.** A local policy applies when any surface path matches its `scope`. It may add rules and checks, never remove them. Its `checks` join with the root's by id.
- **Strictness** (the `⊑` preorder, per parameter):

  | Parameter | Stricter means |
  | --- | --- |
  | `delta` | smaller |
  | `n`, `executions`, `proptest_cases` | larger |
  | `selection` | a superset |
  | `timeout_secs` | not ordered |
  | anything else | equal values only |

- **Join.** Two params for one id join to the stricter value per key. The join is a conflict when any key is incomparable. An unordered key with different values is incomparable, and so is a pair of selections where neither contains the other. A key present on only one side is kept.
- **`r_eff`.** The joined checks of all triggered root and local obligations, sorted by id. On a conflict it is `null`, `conflict` is `true`, and nothing downstream holds.

### 1.2 Predicates

- **`authentic`.** `evidence.producer` is in `trusted_producers`, and `integrity` is `valid`.
- **`applicable`.** `evidence.subject == candidate_tree`, `evidence.policy == policy.digest`, and `evidence.base == base`. Every `required_inputs` name must also be present in `evidence.inputs` with a value that is neither empty nor `"unknown"`.
- **`complete`.** Every check in `r_eff` has exactly one entry with the same id, and `params(check) ⊑ params(entry)`. No entry id repeats.
- **`passed`.** Every check in `r_eff` has its entry's `outcome == "passed"` and kind validity:

  | Kind | Validity |
  | --- | --- |
  | `test` | `selected_tests > 0` |
  | `comparison` | `performed == true` |
  | `corpus` | `corpus_digest == params.corpus_digest` |
  | any other kind | none beyond `passed` |

  `inconclusive` is never `passed`.
- **`eligible`.** All of `authentic`, `applicable`, `complete` and `passed`.

### 1.3 Exceptions

- **The exception object.** `{issuer, authentic: bool, subject, base, policy, waived: [ids], issued_at, expires_at, revoked: bool, logged_at, reason, compensating_control, follow_up}`.
- **`valid_exception`.** `null` when `exception` is `null`. Otherwise it is true only when all of the following hold:
  - `authentic`, `applicable` and `complete` hold, and the exception's own `authentic` is true.
  - `subject`, `base` and `policy` bind to `candidate_tree`, `base` and `policy.digest`.
  - `issuer ∈ exception_authority`.
  - `waived` covers every failed check and contains none of `non_waivable`.
  - `issued_at <= now < expires_at`, and the exception is not revoked.
  - `reason`, `compensating_control` and `follow_up` are non-empty.
  - The issuer is not the `accountable_change_authority`. **(choice)** Under `ECC-Solo`, an issuer equal to the accountable authority is accepted only when `logged_at` is present and `now >= logged_at + cooling_off_hours`. That is the append-only log with a cooling-off delay.
- **`merge_allowed`.** `acceptor ∈ acceptance_authority`, and either `eligible` or `valid_exception` holds.

## 2. Record-lint fixtures

A fixture is `xtask/tests/corpus/records/<id>/`, holding one record file and an `EXPECTED.json`: `{"id", "record": "<file name>", "kind": "evidence | task | acceptance", "accept": bool, "reasons": ["<code>", …]}`. The lint must produce exactly those reason codes, as a set; an accepted record has none. Reason codes:

| Code | Meaning |
| --- | --- |
| `schema.missing_field` | A required field is absent |
| `schema.wrong_type` | A field has the wrong type |
| `schema.unknown_field` | Top level only, for task and acceptance records |
| `evidence.duplicate_check_id` | Two entries share an id |
| `evidence.missing_required_check` | An L0 check id in `.rha/policy.toml` is absent |
| `evidence.passed_nonzero_exit` | `passed` with an exit status other than 0 |
| `evidence.passed_invalid_kind` | For example `test` with 0 selected tests |
| `evidence.failed_without_error_class` | `failed` with no error class |
| `evidence.not_run_without_reason` | `not_run` with no reason, or with an exit status |
| `evidence.not_applicable_without_rule` | `not_applicable` with no rule id and rationale |
| `evidence.inconclusive_outside_comparison` | `inconclusive` on a kind other than `comparison` |
| `digest.malformed` | Not `sha256:` plus 64 lowercase hex, or a bare 64 hex where the field requires it |
| `revision.malformed` | Not 40 hex |
| `timestamp.malformed` | Not RFC 3339 with an offset |
| `digest.mismatch` | A cited file digest differs from the file's bytes, which the fixture supplies beside the record |


## 3. CHG-019.1 amendment: fixture shape (after the data)

**Disclosure:** ledger `CHG-019.1-contract-amendment`, task decision `verifier-contract-amendment`. This section is written after the original 152 fixtures and P-B's review repairs. It records the shape those repairs enforce, including choices the original contract did not fix. V001–V152 and their grading remain unchanged. Separate malformed cases register this amendment; no claim is made that these rules preceded P-B's data.

Before evaluating predicates, a fixture must have these types:

- `policy`: string `digest` and `profile`; arrays `checks`, `rules`, `default_obligations`, `non_waivable`, `trusted_producers`, `required_inputs`, `exception_authority`, `acceptance_authority`; `cooling_off_hours` an unsigned JSON integer representable as u64.
- Each check in root/local `checks` has string `id`, string `kind`, object `params`. Each rule has string `id`, string `scope`, and string-array `requires`. All policy identity/obligation arrays above contain strings.
- `local_policies` is an array; each item has string `scope`, arrays `checks` and `rules` with the shapes above.
- `base`, `candidate_tree`, `acceptor`, `accountable_change_authority`, and `now` are strings. `surface` is a **non-empty array of strings**. Non-empty string content is not imposed by this shape rule.
- `evidence` has string `producer`, `integrity`, `subject`, `policy`, `base`; object `inputs`; and array `entries`. Every entry has string `id`, `kind`, `outcome`, and object `params`. If present, `selected_tests` is a JSON number, `performed` is boolean, and `corpus_digest` is string.
- The `exception` key is required, either null or an object. An object requires strings `subject`, `base`, `policy`, `issuer`, `issued_at`, `expires_at`, `reason`, `compensating_control`, `follow_up`; booleans `authentic` and `revoked`; and string-array `waived`. `logged_at` is optional at the shape layer, but must parse as a timestamp when the self-issued Solo exception path needs it. An absent exception key is not an explicit null.

The first malformed member is reported under `malformed`; `r_eff` is null, `conflict` false, and all predicates and `merge_allowed` false. `valid_exception` is null when the supplied exception is absent/null, otherwise false. The member-order diagnostic precedence follows the order listed above, except that policy fields are checked in their order in §1's example, evidence entry fields precede the acceptor/accountable/now fields, and exception fields come last. Corpus cases isolate one defect each so no precedence ambiguity grades a case.

**Whole test counts:** the `test` validity row in §1.2 means an unsigned JSON integer representable as u64 and greater than zero. Negative and fractional numeric counts are well-formed fixture input but fail `passed`; V031 remains well formed. They are not rounded. This discloses P-B's already-recorded divergence from its pinned reference for a fractional count.

## 4. CHG-019.1 amendment: record schemas and citation notes

[Record schema contract](record-schema-contract.md) fixes the previously unregistered required fields/types/allowed keys by mechanical derivation, with malformed cases before schema or new lint code. It adds `schema.invalid_value` for unsupported enumerations/versions; extends `schema.unknown_field` to closed policy objects; and registers H4, markdown and H5 corpus-report shapes separately from L0 evidence. Existing semantic codes and R001–R080's exact expected sets remain unchanged.

A cited file that neither the record's directory nor its git subject can resolve is explicitly `citation.unchecked` in output. This is a note, not a rejection code: it does not turn a synthetic unknown subject or historic missing raw log into a new frozen-corpus failure. Such a result is printed as accepted **with unchecked citations**, never bare accepted. It cannot support a claim that every digest was checked.
