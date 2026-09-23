# Observed-shape inventory and compact cases, format 1

This is an observation format, **not JSON Schema**. Neither Python program is a
validator. The parent translates the registered inventory into its production
structural representation only after registration.

## Source records

`SOURCE-INDEX.json` lists every candidate source from the pinned revision, its
byte count, SHA-256, family (or null for excluded payloads), and exact snapshot
path. `sources/` contains the original bytes. TOML is decoded with Python 3.11
`tomllib`, then recursively converted into JSON-compatible values; a native TOML
date/time, if encountered, is its ISO string. No source is edited or normalized
in place. JSON output is UTF-8, sorted object keys, two-space indentation, one
final newline, no NaN or Infinity. No source references are followed.

The source tree digest hashes the UTF-8 concatenation of sorted
`<original repository path> <sha256>\n` lines for all 119 candidates. The three
BDR candidate payloads have no supported discriminator and contribute no shape.
They remain in the source index so exclusion is auditable.

## Semantic paths and observations

`INVENTORY.json.families[family].nodes` maps semantic paths to observations. Paths
use JSON Pointer escaping (`~0` for tilde, `~1` for slash), root `""`. A `*`
segment pools **array elements only**; it never means arbitrary record keys.
At a dynamic keyed map, `{}` pools values across arbitrary names. Separately
owned report and witness payloads retain their documentary field paths instead.
These two reserved path segments do not occur as literal record keys here.

Each node has sorted `types`, `type_observations`, contributing `source_paths`
and `source_count`. Types are `null`, `boolean`, `integer`, `number`, `string`,
`array`, `object`. Python booleans are classified before numeric types. Integer
and floating JSON number are distinct **observations**. When translating to
2020-12, JSON Schema's numeric domain semantics still apply: `number` includes
integers, and an integral numeric value can satisfy `integer`. No special
lexical-number restriction is introduced by this inventory.

For object observations only (ignoring null and other types at the same path),
`observed_required_keys` is the intersection and `observed_allowed_keys` the
union. `object_observations` and `empty_object_observations` disclose denominators.
An intersection starts with the first actual object's key set, never an empty
accumulator. Empty arrays are not empty objects and do not erase item shapes.
An actually observed empty object, if any, participates in its intersection.

For `object_mode = closed_record`, `required_keys` and `allowed_keys` are the
effective key constraints. Missing optional parent objects do not make their
children optional when those objects are present. Types apply before deeper
member/value checks: a scalar in place of an object yields wrong_type, not all
of that absent object's child requirements. The same principle avoids spurious
enum errors on a value whose type is already wrong.

For `object_mode = open_map`, `required_keys = []`, `allowed_keys = null` means
arbitrary names, **not a null-valued property**. `map_value_domain = string`
requires each value to be a string; `any` imposes no value or descendant
constraints. The boundary's observed object/nullable types are still enforced.
Observed incidental key sets are documentary, never required or allowed-key
lists for validation. All nodes below an open boundary have `enforced = false`;
ignore their shapes when translating. `dynamic_maps` is the exhaustive exact
family/path boundary list. There is no blanket rule that opens every object.

Every array records its number of observations, empty observations, and pooled
item observations. `item_domain = unconstrained_unobserved` means no element
was seen anywhere in that semantic path's scope; its items must remain
unconstrained. Otherwise use the `/*` node. There is no inferred minItems,
uniqueness, numeric bound, semantic regex, or enum for ordinary strings.

## Variants and explicit contract extensions

`variants` replace a subtree's **pooled** nodes with the selected branch nodes;
branch keys are absolute semantic paths. Selection is deterministic and uses
key presence (including present null) or the explicitly named JSON type:

1. `/agent_context`: `any_key_present` among `source`, `provenance` selects
   `sourced`, otherwise `legacy`.
2. Inside sourced context only, `/agent_context/provenance`: presence of any of
   `harness`, `task_scope`, `permissions`, `budgets`, `reasoning_effort` selects
   `full`, otherwise `compact`.
3. `/change_claim`: presence of `intent` **or** object-valued `source` selects
   `sourced`, otherwise `unsourced` (null task, null source, note).

Apply the context replacement first, then its nested provenance replacement;
do not reintroduce pooled children from the other branch. An empty provenance
object selects compact and lacks both required keys. Full model_id is allowed:
it was observed in full records and is not a forbidden key. Compact string
instruction items do not become valid full citation objects by pooling.

Branch failures are only the selected branch's ordinary required/type/unknown
field errors. Do not aggregate rejected alternatives' errors. Keys belonging
only to another branch yield unknown_field; missing selected-branch keys yield
missing_field; wrong selected-member types yield wrong_type. No generic
invalid_value is added for provenance/claim branching.

The H4 `exactly_one_key_present` rule is additive to the pooled case record:
exactly one of `fixture_path`, `fixture_inputs_sha256` must be present, regardless
of value type. Neither gives missing_field; both gives invalid_value. A present
wrongly typed identity still gives wrong_type. Neither property is made
individually required by pooling. The digest-map values are strings.

After branch replacement, merge each `extensions` entry at its path. `types`
replaces the type constraint at that path; `values` is an exact finite domain;
`required_in_parent` explicitly adds or removes the field from parent
requiredness. An extension field is added to the parent's allowed keys. These
entries own schema versions, supported JSON discriminators, the five actual
check/summary outcomes, and optional L0 policy_rule_id/rationale strings.
All other key/type observations stand. Documentary held_out.outcome is an
ordinary string. H4 level is an ordinary string, including `crate` and `module`.
H4 versions 1 and 2 use the family's pooled envelope; the contract does not
require an additional version-dependent split. H5 corpus is string-or-object;
required citation keys apply only in its object variant.

`identification` records the structured family discriminator. Direct fixture
grading uses `case.family` even if a discriminator was deleted. JSON missing,
unsupported, and mistyped discriminators give missing_field, invalid_value, and
wrong_type respectively. TOML task id/mode, acceptance acceptor, and policy
authority/lanes are structural identification members; no finite ordinary
string domain is inferred from their samples.

## Mutation materialization and grading

`CASES.json` contains a `baselines` dictionary and a `cases` list. Each baseline
names one hashed source snapshot and optional ordered setup `mutations`; it
does not duplicate the record. Each case has `id`, `family`, `baseline`, ordered
`mutations`, sorted deduplicated `expected_schema_codes`, and `purpose`.

Start with a fresh deep copy of the parsed baseline source, apply baseline
mutations, then apply case mutations in order. These are **literal JSON
Pointers**, with decimal zero-based array indexes, never `*` or `{}`. Decode
`~1` then `~0`. `remove` deletes an existing object member or array element;
missing targets are generator errors, not no-ops. `set` replaces a value or adds
a final object key; its parent must already exist. Array `set` only replaces an
existing index; it neither inserts nor appends. An empty pointer with `set`
replaces the root. Root removal and `-` append are not defined. Values are deep
copied. No substitutions, paths, commands, citations or environment variables
inside strings are interpreted.

Expected codes grade **only structural schema.* reasons**, as a set. Parse,
required, type, unknown-key, and unsupported-value failures map to the contract
codes. Semantic digest/revision/outcome defects do not add structural codes.
Empty expected sets are structural controls, not assertions of authenticity,
eligibility, successful checks, or lint acceptance. Named `RS-cli-*` cases are
available for later CLI regressions; generation did not execute that CLI.
