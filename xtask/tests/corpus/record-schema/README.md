# CHG-019.1 independent record-shape registration

This registration freezes observations and structural mutation expectations
before the parent writes schemas or new lint code. `derive.py` reads only the
pinned source objects (or their retained snapshots) and writes an observation
inventory. `cases.py` constructs recipes and materializes JSON Pointers. Neither
program generates JSON Schema or evaluates a schema/lint result.

The machine-consumable format, exact map boundaries, branch selectors, mutation
semantics and diagnostic interpretation are defined in `FORMAT.md` and
`INVENTORY.json`. Exact final counts and hashes are in `registration.toml`.

## Scope and provenance

The source revision is `99a74b042842b4d869ef341f58fdfcafb9c96e84`. There are 119
candidate source snapshots: 11 tasks, 10 acceptances, one policy, 75 L0 evidence
records, 12 H4 reports, six H5 reports, one markdown report, and three excluded
BDR candidate payloads. The 116 supported records each have an unmodified
structural control. This includes the requested early/current tasks and
acceptances, both current L0 context forms, early/current H4 and H5, and markdown.

Independent counts are 66 sourced contexts and nine legacy direct contexts;
inside sourced contexts, 60 full provenance objects and six compact objects.
The corresponding claims comprise 66 full sourced and nine unsourced forms.
The 420 H4 cases comprise 280 fixture_path-only and 140
fixture_inputs_sha256-only identities; none has both or neither. Regression
recipes explicitly exercise both invalid identity states and empty/mixed
provenance. No branch-error set is inferred from failed alternative branches.

Generation used gpt-6-astra, reasoning effort xhigh, sample 1, without a PRNG.
One earlier attempt was interrupted with SIGINT before this staging directory
or any payload existed, to clarify the contract. Work resumed in the same
independent session. All three exact user prompt files are retained and hashed.
The amended current contracts are retained as governing inputs, separately
from the frozen derivation data. Frozen plan/spec excerpts were read; their
complete committed documents are retained as reference inputs. No schemas,
lint implementation, verifier implementation, sibling project, or held-out
cases were read. Strings mentioning such paths in permitted source records
were treated only as data, never followed. No Git/JJ mutations were run.

The third prompt fixes verifier §4.1 wire labels before registration/commit or
production grading. This record registration's inventory, cases and source
snapshots remain unchanged; its governing verifier-contract snapshot and
provenance reflect the clarification. `metadata/GENERATION-HISTORY.json`
retains two failed exploratory inspection commands, two incomplete section
extractions and the pre-payload interruption. No completed deterministic
generator self-check failed and none was observed interrupted. The registrations
report these categories separately; this is the same session and sample 1.

## Reproduction

From the repository root, using Python 3.11 or later:

```sh
python3 -B target/m2/schema-generation/record-schema/derive.py --from-git --check
python3 -B target/m2/schema-generation/record-schema/derive.py --check
python3 -B target/m2/schema-generation/record-schema/cases.py --check
python3 -B target/m2/schema-generation/record-schema/cases.py --materialize RS-cli-unknown-outcome
python3 -B target/m2/schema-generation/record-schema/register.py --check
```

Omit `--check` on derive/cases only to rewrite their deterministic generated
artifacts inside this registration. `--from-git` uses only git ls-tree/show at
the pinned revision; without it, source bytes and their hashes are checked from
the retained snapshots. The final command parses JSON/TOML, checks source and
prompt hashes, and checks registration tree digests; it does not run schema
validation. Production lint/verifier and test runners were never invoked.

## Limits and explicit non-claims

- Structural compatibility is not semantic acceptance, authenticity, eligibility,
  citation verification, or a rerun of a corpus. Historic digest/revision errors
  and documentary statuses cannot change these structural-only expectations.
- Three BDR reports have no supported discriminator and are excluded from
  derivation. No shape or enum is guessed for future report families.
- Every entirely unobserved array-item domain is enumerated in
  `metadata/UNOBSERVED-ARRAYS.json`; its items remain unconstrained. Future
  tightening requires a new contract and registration. Known list records
  remain pooled records; empty arrays do not erase their shapes.
- Maps stop at the exact paths in `INVENTORY.json.dynamic_maps`. Structured
  artifact_identity, subject, evidence_inputs, and policy lanes are not opened
  wholesale. No top-level literal `inputs` map was observed among these record
  families; verifier fixture evidence.inputs is separately specified in B.
  Embedded report subject/input dictionaries lie inside the open report
  boundary. Report/witness/finding payload descendants are inventory only.
- L0 selection_counts is a nullable measurement-name dictionary; CI environment
  names and check params are semantic keys. Path-to-digest maps require string
  values; arbitrary map values elsewhere remain unconstrained by the contract.
- Record-policy cooling_off_hours has an observed integer type but no
  contract-defined minimum. Its negative integer structural control must stay
  separate from verifier fixture §3's unsigned-u64 rule. No minimum is invented.
- Ordinary strings are not enumerated. H4 level accepts module; documentary
  held_out.outcome accepts not_part_of_this_run; only actual L0 check and corpus
  summary outcomes use the five states. The corpus does not claim to test real
  module extraction or any held-out case.
- Named CLI regressions use a current all-passed CI baseline with citation
  arrays cleared by an explicit recipe to avoid old citation mismatches. They
  are synthetic inputs, not newly observed evidence. Their semantic isolation
  follows the published contract and source values; production lint was not run.
- Mutation recipes operate on parsed JSON-compatible values. They do not grade
  TOML textual formatting, malformed JSON/TOML syntax, duplicate parser keys,
  or the spelling of a JSON number such as 1.0. Parse-error mapping is recorded
  in the contract but no invalid-byte fixture is invented in this recipe format.
- Existing R001–R080 and V001–V152 are preserved; these artifacts do not regrade
  them. Corpus-report structure does not authenticate checker output.
