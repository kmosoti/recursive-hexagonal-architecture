# Frozen cases and pre-implementation diagnostic contract

Authority: `xtask/tests/corpus/manifest.toml`, read without editing. Each staged
headline directory has its exact parsed `[[case]]` in `contract.json`.
`headline-contract.json` also preserves all module cases and the entire grading
table. No module case in the frozen manifest has `expected_findings`. The lone
registered extra finding elsewhere in the manifest belongs to crate case C13;
it does not authorize extra findings on module fixtures. Every witness key,
including M20's `extraction` and M05's `depth`, remains binding. Diagnostic
representation is fixed by `docs/architecture/module-check-contract.md`, read
during the pre-registration correction. Nothing here changes headline grading
or grades an implementation.

## M01 resolved by structured D4 subfacts

M01 explicitly requires `constraints` to use `crate::ordering::Wave`, while the
component declaration permits only `ordering = ["constraints"]`. Its witness
requires the cycle `constraints -> ordering -> constraints`. The fixture supplies
the necessary *actual* reverse dependency in an ordering function's argument
type. An allow-list entry alone is not a source dependency and cannot close the
cycle.

The forward edge also violates D4. The diagnostic contract requires one cycle
finding for its cyclic SCC and retains this undeclared edge in that finding's
`undeclared_edges`, with `modules.undeclared_dependency` in `subsumed_rules`.
There is no redundant top-level undeclared-dependency finding. The forward D4
fact is preserved, not declared conformant or waived. Undeclared edges outside
cyclic SCCs remain independent findings; foreign-internal consequences remain
independent even inside a cycle.

The canonical witness starts at the least component, `constraints`, and is
`["constraints", "ordering", "constraints"]`. The fixture, its one-way
permissions, and the manifest's expectations remain unchanged. The first
proposal incorrectly treated flat raw predicate findings as the only permitted
diagnostic representation; the pre-registration contract review corrected the
optional rule projection. It did not regrade a production result.

## M05 depth is three

The manifest fixes `scoring = "x::ordering::scoring"`, a reference to
`crate::constraints`, and `depth = 3`. The staged crate is named `x`, so its
canonical source path is exactly that value. The diagnostic contract defines
depth as the number of canonical component-path segments, including the crate
root as depth one: `x`, `ordering`, `scoring` gives **three**. The target is the
parent's sibling, which is D2 regardless of visibility or an allow entry.
Containment classification happens before direct-sibling dependency checks;
this edge does not enter the sibling graph. The raw reference still only
extracts source/target paths and does not manufacture headline witness fields.

## Fixture isolation choices, with no grading edits

- M12–M18, M20 and M21: both actual component directions are explicitly allowed,
  while `deny.cycles = true`. Their seeded descriptions do not fix allow-lists.
  This isolates their registered cycle findings from undeclared-dependency
  facts. M01 instead retains its undeclared edge as a cycle subfact. These are intentionally
  architecturally violating declarations/graphs; all sources are legal Rust.
- M02: `model` is declared; `constraints -> model` is not permitted; there is no
  actual reverse edge. Only the undeclared edge is seeded.
- M03/M04/M19: no child-to-child reverse dependency is added. The root item is
  the upward target, regardless of its visibility.
- M05: `constraints`, `ordering`, and nested `scoring` are declared. The fixture's
  existing literal `scoring -> constraints` permission remains unchanged; the
  original author included it to isolate D2. Under the diagnostic contract,
  containment classification already excludes this edge from sibling checks,
  and the permission cannot authorize it. The source uses the module itself as
  required.
- M06: `ordering -> constraints` is permitted; its public `rules::Rule` path
  still crosses below the facade. Rust privacy does not mask the violation.
- M10: `part` is explicitly a component and may reference `constraints`; its
  `#[path = "elsewhere.rs"]` file imports `crate::constraints::rules`, exactly
  the witness target, without additionally naming `rules::Rule`.
- M11: the sole forward edge is inside `constraints::tests`, inherited
  `test_only = true`. It is a listed fact, never a finding.
- M17: the literal nested use tree is in a nested module to avoid defining and
  importing `Rule` twice in one Rust namespace; both edges still collapse to
  the registered components.
- M21: edition 2015 makes the exact bare `use ordering::Wave;` sibling import
  legal without adding a prefix or another import. The manifest fixes no
  edition. Supplementary edition-2021 fixtures separately exercise uniform
  paths through legal lexical module bindings. The reference never grants an
  arbitrary edition-2021 sibling fallback.
- M09: `std`, `core`, and an `extern crate std as external` alias exercise
  external paths without third-party dependencies or package installation.
- All fixtures retain `cycles`, `child_to_parent_private` and `foreign_internal`
  set to true. No module fixture's registered rule flags have been disabled.

## Named holes and exclusions

M19 intentionally preserves the explicit root facade path `crate::score` when
it is called from `constraints`. The reference resolves *lexical import
bindings* but does not forward an explicitly qualified facade member through
its re-export. This preserves the manifest's documented asymmetry. It is not
an additional hidden constraints-to-ordering edge.

EM-M01 ignores the macro definition body and reports that limitation; the
invocation has no path-bearing arguments. EM-M02 never reads the include
fragment during extraction; the Rust compiler does read it during legality
checking. Both fixtures have an explicit reverse edge so the hidden path would
complete the intended component cycle.

EM-M03 obtains a `model::Value` from an ordering factory and calls a method from
a glob-imported ordering trait. The caller's source never names `model`. The
reference finds the visible glob/factory paths and the ordering-to-model path,
and reports its missing method/type inference. The additional implicit caller
dependency on `model` remains the documented hole. Permissions allow both
visible directions but do not allow `constraints -> model`.

L-M01 is the live site crate: no substitute was authored and no product source
was read. X-M01 requires an external reference-tool run involving that live
crate: no such run was attempted, and tool absence is not asserted. Neither
case is replaced by the supplementary corpus.

The parent still owns committing these artifacts before implementation and all
subsequent independent validation, implementation, acceptance, and grading.
The two representation questions above are resolved by the pre-implementation
contract. Historical proposal identity and the correction are retained in
`provenance/README.md`; no source corpus or extraction oracle was changed.
