# CHG-019.1 supplementary verifier shape fixtures

These S-series fixtures register verifier-contract §§3 and 4.1 independently of the
production implementation. Existing V001–V152 remain untouched. No production
or reference evaluator was read, imported, built, or run. `generate.py` copies
seven original V baselines from the frozen revision, performs explicit single
mutations, and assigns outputs from the contract and baseline expected values.

Exact counts and hashes are in `registration.toml`. `metadata/RECIPES.json`
identifies every baseline, single mutation, precise malformed member (if any),
and expectation derivation. `metadata/CASE-INDEX.md` is a compact review index.
Only S fixtures are JSON files at the top level; source/index/helper JSON lives
below `sources/` or `metadata/` so the original top-level runner cannot mistake
it for a new fixture.

## Derivation and coverage

The seven byte-for-byte baselines at
`99a74b042842b4d869ef341f58fdfcafb9c96e84` are V015 (local checks/rules), V031
(negative selected_tests), V127 (ordinary passed test), V129 (comparison),
V130 (corpus), V147 (self-issued Solo exception), and V148 (independent
exception without logged_at). Their exact original hashes and counts are
recorded in `metadata/SOURCE-INDEX.json`; no fixture files in the original
corpus were modified. Original expected objects are retained in the snapshots.

The supplementary cases cover absent/type-invalid policy and evidence
containers; all required root strings; every policy member; each identity or
obligation array and its non-string items; root/local checks and rules,
including nested required keys and malformed items; the local policy container
and each member; absent, non-array, empty and non-string-item surface; all entry
required fields; optional kind-specific member types; all required exception
members, null/object controls, and the required exception key itself.

Cooling-off negative, fractional, boolean, null, string, and overflowing values
are malformed. Zero and u64::MAX are shape-valid controls on the null-exception
baseline. Numeric selected_tests values that are negative, fractional, zero or
above u64::MAX remain **well formed**, with passed/eligible/merge_allowed false
and no malformed key. Positive whole counts retain the registered success.
Optional kind-specific members may be absent at the shape layer but then fail
their kind's Passed criterion. Unknown string outcomes in these verifier
fixtures are well formed but fail Passed; record-schema outcomes in A have a
different, explicit five-state structural rule.

Malformed outputs have null r_eff, conflict=false, every predicate false, and
merge_allowed=false. valid_exception is null for absent/null supplied exception
and false for any supplied non-null exception. Only one defect is introduced
per malformed fixture, so diagnostic precedence is never graded across defects.
Well-formed fixtures copy original outputs and change only the explicitly
justified predicates; no evaluation algorithm was implemented to assign them.

## Diagnostic spelling and limits

§4.1 now fixes P-B's grouped wire labels. Nested check, rule, local-policy and
evidence-entry defects use `policy.checks`, `policy.rules`, `local_policies` and
`evidence.entries`. String-array item defects use their containing group.
Exception leaves use exactly the corresponding published label:

- `exception.subject, base, policy, issuer, issued_at, expires_at`
- `exception.authentic, revoked`
- `exception.waived`
- `exception.reason, compensating_control, follow_up`

An absent/wrong exception container uses `exception`. An absent/wrong whole
policy or evidence container fails the first listed field check, labelled
`policy.digest` or `evidence.producer`. `metadata/RECIPES.json.malformed_member`
and CASE-INDEX retain the precise offending JSON Pointer, including `/policy`
and `/evidence` for whole-container defects. Thus `/policy/checks/0/id` remains
the pointer while `policy.checks` is the output label. The CASE-INDEX displays
both separately; the recipes and pointers are unchanged.

The earlier staged draft used precise leaf labels while §3 left their wire
spelling underdetermined. The parent published §4.1 before registration/commit
or production grading of this corpus. This correction changes only the
supplementary expected.malformed strings, not mutation recipes, inputs or
predicate expectations. `metadata/WIRE-CLARIFICATION-AUDIT.json` retains the
previous labels and before/after fingerprints. Production implementation was
not read or run to choose these labels. The former wire-format ambiguity is
resolved by the published contract; production equivalence has not been run.

`logged_at` is shape-optional and has no unconditional member-type requirement
in §3. An absent or unparseable value on the self-issued Solo path makes the
exception invalid without a malformed field. An independent issuer ignores
that unused member. String content is otherwise governed by the predicates,
not inferred shape restrictions; empty reason/control/follow_up examples are
well formed but invalidate the exception. Shapes do not test real signatures,
append-only logs, permission isolation, or held-out data.

The contract does not settle whether a numeric token such as `1.0` is a JSON
integer for verifier u64 conversion. These fixtures do not invent that lexical
expectation: integer controls use integer tokens, fractional controls have a
nonzero fractional part. Empty string surface elements are allowed by the
shape rule, but their interaction with trigger/default obligations is not
graded by this supplementary shape corpus. Multiple simultaneous malformed
members, parser syntax failures, and unknown extra fixture keys are also not
graded. The original corpus continues to own its semantic scenarios.

## Generation provenance and reproduction

Generator gpt-6-astra, xhigh, sample 1; no PRNG and no random seed. One attempt
was interrupted before payload creation for contract clarification, then the
same independent session resumed. All three exact user prompts and both current
amended contracts are retained with computed hashes. Frozen plan/spec are
reference inputs. No sibling/held-out material, schema code, lint code or
verifier code was accessed; source strings referring to such paths were never
followed. No version-control mutation was performed.

`metadata/GENERATION-HISTORY.json` preserves the entire observed failure history:
two failed exploratory inspection commands (missing staging path, then a
historical H4 fixture-key assumption), two incomplete section extractions, and
the pre-payload interruption. No completed deterministic generator self-check
failed, and no deterministic self-check was observed interrupted. These counts
are explicit in the registration rather than describing the whole attempt as
failure-free. The wire-label correction is pre-registration clarification,
not an after-run change to grading or a new sample.

From the repository root, Python 3.11 or later:

```sh
python3 -B target/m2/schema-generation/verifier-shape/generate.py --from-git --check
python3 -B target/m2/schema-generation/verifier-shape/generate.py --check
python3 -B target/m2/schema-generation/verifier-shape/register.py --check
```

Omitting `--check` regenerates only these staged outputs. Git reads are pinned
`git show` calls. Without `--from-git`, reproduction uses retained source
snapshots. Validation is limited to parsing, source/prompt hashes, deterministic
construction and tree digests. It is not a claim that a verifier accepts the
fixtures or reproduces their expected outputs.

The tree digest hashes sorted UTF-8 `<relative path> <sha256>\n` lines for every
payload in this directory except `registration.toml` and
`GENERATOR-PROMPT.md`. README, this generator, metadata, original source
snapshots, governing/reference inputs, `GENERATOR-CORRECTION.md`, and
`GENERATOR-WIRE-CLARIFICATION.md` are included.
