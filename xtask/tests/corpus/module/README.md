# Independent P-C supplementary module corpus

These are staging artifacts from the independent generator session, before
exposure to a real extractor. No product/checker implementation was read or
run. Everything is under `target/m2/module-generation/`; nothing was committed.
The parent must commit this registration before real extractor implementation
or differential grading. The headline authority remains the unchanged
`xtask/tests/corpus/manifest.toml`: M01–M21, L-M01/L-M02, EM-M01–EM-M03 and X-M01.
This corpus supplements that contract, and defines no relaxed headline grading.
The optional diagnostic projection follows the pre-implementation
`docs/architecture/module-check-contract.md`. Raw extraction remains the primary
supplementary comparison. Its sources and expected edges were not changed by
the pre-registration diagnostic-contract correction.

## Contents

- `generate.py`: seed **20260922**, fixed SplitMix64 arithmetic, 256 source maps.
  Each `random/Rnnnn.json` contains the crate name, edition, and UTF-8 files
  keyed by relative path. It contains no expected edge list. Layout/template
  counters are provenance only and are never passed to the reference.
- `reference.py`: independent Python standard-library lexer, recursive parser,
  module walker and lexical import resolver. It imports no generator or
  production code. The reference reads materialized `.rs` files, not source
  annotations or a planned graph. Each expected JSON was obtained by invoking
  its JSON protocol in a separate process.
- `random/Rnnnn.expected.json`: reference output for every sampled graph.
- `random-rule-expectations.json`: optional flat-component contract consequences
  computed solely from extracted edges and fixture rules. Not a headline grader.
- `headline/`: 25 separately compilable staged fixtures: M01–M21, L-M02 and the
  three EM-M cases. `contract.json` is the exact frozen case projection;
  `reference.json` records this reference's extraction, not a production result.
- `CONTRACT-NOTES.md`: M01's structured undeclared-edge subfacts, M05's depth
  definition, M21's edition disclosure, isolation choices and expected holes.
  No headline expectations were amended.
- `provenance/`: the first proposal's original registration, digest inventory,
  and historical optional rule output, plus the correction record. Historical
  files are not active expectations. The first proposal used flat raw predicate
  findings; pre-registration contract review corrected their representation
  before any real extractor implementation or production grading.
- `test_reference.py`, `check.py`, `reproduce.py`: reference hand checks, actual
  Rust legality checks, extraction replay, and byte-for-byte regeneration.
- `checks.json`, `coverage.json`, `reproduction.json`, `registration.toml`,
  `SHA256SUMS`: measured counts, versions and cryptographic content inventory.
  `correction-audit.json` records raw-oracle preservation, D4-fact conservation,
  and independent exhaustive checks of the corrected SCC diagnostics.

## Exact reproduction commands

Run from `/home/kmosoti/projects/rha-m2`, with Python 3.11+ (tomllib) and the
already installed Rust toolchain. There are no third-party Python or Rust
dependencies. These commands write only below the staging directory:

```sh
python3 -B target/m2/module-generation/generate.py
python3 -B target/m2/module-generation/headline.py
python3 -B target/m2/module-generation/check.py
python3 -B target/m2/module-generation/reproduce.py
python3 -B target/m2/module-generation/register.py
```

`check.py` fails nonzero on a failed compiler command, reference replay,
contract-projection comparison or hand check. It compiles each random source
map with `rustc --emit=metadata --crate-type=lib` both normally and with
`--cfg test`. It runs `cargo check --offline --all-targets` on every headline
fixture, with an isolated staging target directory, then removes build output.
`Cargo.lock` files remain with the headline fixtures. Checks are generator
self-checks and compiler legality evidence; they are not independent validation
of a production implementation.

`reproduce.py` regenerates all random maps/expectations and headline
sources/projections in a temporary staging subdirectory and compares their
bytes and complete file sets, excluding compiler-produced lockfiles. It removes
that subdirectory after comparison. `register.py` must run last: it hashes all
durable artifacts, the supplied prompt, and the frozen source contracts.
It also binds the exact correction prompt, the diagnostic contract, and the
first proposal identity. To replay this correction without rewriting the
canonical source maps, headline sources or primary expected outputs, run only
`check.py`, `reproduce.py`, then `register.py`; regeneration during
`reproduce.py` is confined to a temporary staging subdirectory.

Verify the file inventory without regenerating it:

```sh
cd /home/kmosoti/projects/rha-m2/target/m2/module-generation
sha256sum --check SHA256SUMS
sha256sum SHA256SUMS registration.toml
```

The SHA256 of the **bytes of SHA256SUMS** is `registration.content_sha256`.
Each inventory line is `<64 lowercase hex digits><two spaces><relative POSIX
path><LF>`, sorted by path. `SHA256SUMS` and `registration.toml` exclude themselves
to avoid a recursive digest. The registration's random/headline digests use the
same byte stream filtered by the corresponding directory prefix. Do not infer
the digest from a tar stream or filesystem metadata. Both prompt hashes, the
manifest hash and the diagnostic-contract hash bind their exact read-only file
bytes. Counts and all hashes come from scripts. The original content identity is
retained as `first_proposal_content_sha256` and by the archived original inventory.

## Deterministic extractor protocol

One UTF-8 JSON object on stdin; one JSON object and LF on stdout. No logging to
stdout. `root` is the materialized library source path, not its directory.
`crate_name` is the Rust crate identifier (hyphens normalize to underscores).
`edition` defaults to `2021`; `externals` defaults to an empty list, in addition
to the recognized `std`, `core`, and `alloc` names.

```sh
python3 -B target/m2/module-generation/reference.py <<'JSON'
{"root":"/home/kmosoti/projects/rha-m2/target/m2/module-generation/headline/M20/src/lib.rs","crate_name":"fixture_m20","edition":"2021"}
JSON
```

Output schema:

```json
{
  "schema_version": 1,
  "edges": [
    {"source":"fixture_m20::constraints","target":"fixture_m20::ordering::score","test_only":false,"extraction":"heuristic"},
    {"source":"fixture_m20::ordering","target":"fixture_m20::constraints::Rule","test_only":false,"extraction":"syntax"}
  ],
  "limitations": []
}
```

Exit 0 means extraction completed, **including when limitations are present**;
it is not an architecture pass. Exit 2 with `{"error":...}` means the input
request cannot be processed. A missing/ambiguous module file, unbalanced source,
unresolved path or unsupported construct is reported in `limitations`, never
converted into an architecture finding. For this finite domain, any unexpected
limitation invalidates an exact edge comparison. Absence of limitations is not
a completeness claim for arbitrary Rust.

## Normalization for the implementer

1. `source` names the **lexical module**, starting with the normalized crate
   identifier. Root is the crate identifier alone. An inline module's path is
   semantic, independent of the containing file. A `#[path]` filename does not
   become part of the module's name.
2. `target` is the full normalized module/item path. Resolve leading `crate`,
   `self`, repeated `super`, and lexical import bindings. Expand use trees into
   their leaves; normalize `::{self}` and `::*` to the module path itself, with
   no trailing `self` or `*`. Keep the terminal item name for item paths.
3. Resolve renamed imports, imports of modules, import chains, and unambiguous
   names from glob imports against parsed declarations. A reference through a
   lexical binding uses that binding's declared target, followed by any suffix.
   A **qualified facade member** such as `crate::score` retains its explicitly
   named facade: do not chase the facade's `pub use` to another component.
   That last boundary is required by M19. It also applies to glob-exported
   names: the exporting module remains the named facade.
4. Include references from `use` leaves, expressions, types, impl headers and
   macro argument tokens. Module declarations, comments and string contents are
   not references. Drop paths whose owning module equals `source`. Keep edges
   between different modules of the same component; these may form legal
   intra-component cycles. Drop external paths and local-variable names;
   unresolved nonlocal names are limitations. Module-vs-item type metadata is
   not added to this minimal schema; the implementation has its own module tree.
5. `test_only` is true under a `#[cfg(test)]` module or item, inherited through
   descendants. Test edges are listed rather than discarded. Both test and
   non-test occurrences of the same path remain distinct tuples.
6. `extraction` is `syntax` for the parsed syntax subset and `heuristic` for
   paths found in macro **arguments**. A heuristic edge is not promoted to
   syntax even when its tokens resemble an ordinary function call. If both
   forms occur, retain both tuples. Macro-definition bodies are not arguments.
7. Deduplicate by the complete four-field tuple, then sort by `(source, target,
   test_only, extraction)` using Unicode/ASCII lexical order and false before
   true. Limitations have only `source`, `code`, `detail`; deduplicate and sort
   by that tuple. File offsets, spans, visitation order and absolute file paths
   are not part of normal edge identity. JSON keys are serialized sorted.

Use this normalization for supplementary comparisons. Do not substitute these
fields for the headline manifest's witness schema, cycle path, `depth`, or
`listed_as` requirements. `reference.json` is not a new headline oracle.

## Generated finite syntax domain

All random fixtures use Rust 2021, no dependencies, 3–5 flat declared components,
and a glue root. SplitMix64 samples component counts, permitted-direction edges,
test targets and additional file layouts. Eleven rotating syntax templates
guarantee coverage across the seed; the generator's actual graph choices are
not given to the reference. `coverage.json` records measured template/layout
counts and unique collapsed graph shapes.

The lexer recognizes ASCII identifiers, decimal integers, ordinary quoted
strings, punctuation, balanced delimiters, line comments and nested block
comments. The parser's finite item domain is:

```text
item := attrs? visibility? (mod | use | unit_struct | fn | trait | impl)
attrs := #[cfg(test)] | #[path = "relative.rs"] | #[allow(...)]
mod := mod NAME ; | mod NAME { item* }
use := use PATH_OR_RECURSIVE_GROUP [as NAME] ;  # self and glob leaves allowed
unit_struct := struct NAME ;
fn := fn NAME (NAME : TYPE, ...) [-> TYPE] { statement* } | trait_signature ;
trait := trait NAME { function_signature* }
impl := impl PATH [for PATH] { fn* }
TYPE := unit | primitive | local_unit_type | PATH
statement := use | let NAME [: TYPE] = expression ; | expression [;]
expression := literal | PATH | call | grouping | macro_invocation
```

Generated ordinary bodies use only unit constructors, trivial calls, literals,
and `let` bindings; all recursive compile-time dependencies remain legal Rust.
There are no generics, associated-item paths, destructuring, overlapping glob
exports, local names used before later shadowing, type aliases, conditional
import bindings, or conditional module alternatives. Function-body imports do
not affect function signatures. Imports are resolved after collecting module
declarations, so ordinary forward declarations are supported. The additional
headline forms are `macro_rules!`, `include!`, an extern-crate alias and a method
call; their expansion/inference holes are explicit.

Both `name.rs` and `name/mod.rs`, inline modules, nested file modules, and
file-module `#[path]` declarations are followed. In the generated domain,
`#[path]` is attached to file modules and names simple sibling files; no
path-on-inline-module, escaped filename, symlink, or cyclic file declaration is
generated. Module files must remain inside the materialized fixture directory
(the parent of `src`); a crossing is a limitation. Missing or competing default
files are limitations.

Rust-2021 uniform names must be actual lexical bindings. The generator uses
`use super::cN; use cN::Token as Alias;`, not an invented implicit sibling
fallback. Headline M21 separately uses Rust 2015, whose root-relative bare use
path makes the registered text legal. The reference accepts an edition field
for that purpose; no other edition behavior is generalized.

## Semantic coverage and limits

Every random case has inline/file/`#[path]` modules, self/super paths inside
components, nested modules, a glue-to-children control and listed test edges.
Across the seed, expression, type, impl, use group/tree/self-leaf, rename,
module alias, glob, uniform binding, re-export and heuristic macro paths all
occur. Strings/comments deliberately contain false path tokens. Each component
has a legal internal cycle that must disappear on component collapse.

The four equal-size regimes give 64 architecturally legal graphs, 64 cyclic
graphs, 64 acyclic graphs with an undeclared direction, and 64 boundary-violation
graphs (32 foreign-internal and 32 upward references). Rule consequences are
re-derived from reference output, not regime labels. The flat rule helper
excludes test edges, collapses by longest component prefix, checks the allow
matrix, detects root-item and below-facade paths, and computes cycle SCCs using
exhaustive reachability. Each cyclic SCC has one finding with sorted `members`
and a canonical closed `path`: least component start, shortest return cycle
through that start, with lexical order breaking equal-length ties. Only edges
on that witnessed path contribute to its `extraction` flag; a heuristic
occurrence on a witnessed component pair makes the flag heuristic even if that
pair also has syntax evidence. A heuristic edge elsewhere in the SCC does not
change that witness's flag.

All undeclared component edges within a cyclic SCC are retained, including
edges outside its canonical witness, in sorted `undeclared_edges` with `from`,
`to` and `extraction`. A nonempty list sets
`subsumed_rules = ["modules.undeclared_dependency"]`; empty lists remain empty.
Independent undeclared edges remain top-level findings. Diagnostic endpoints
use `from`/`to`, cycles use `path`, and upward findings use canonical component
segment `depth`, following the diagnostic contract. `component_edges` retains
its original `source`/`target` representation and all extraction flags.
`coverage.json` distinguishes top-level finding counts from subsumed D4 facts,
so diagnostic grouping cannot conceal lost facts.

This helper is limited to the generated flat schema with all three deny flags
true; it is not a general module-policy implementation and does not manufacture
headline witnesses. Its generated domain has no associated-item paths or
facade re-export forwarding, and every below-facade foreign path ends in a
known generated internal module's unit type. Its finite depth test is valid
only for those forms; the real contract requires the discovered module tree,
not segment counting for arbitrary Rust item ownership. Root references use
component depth including the crate root; nested headline M05 is documented
separately and is not evaluated by this flat helper.

Limitations are deliberate: no macro expansion (definition bodies ignored), no
include expansion, no method/type inference, no procedural macro expansion,
no general cfg evaluation, no generic/associated type resolution, no arbitrary
Rust syntax, no full Rust privacy/namespace model, and no forwarding through
explicit re-export facades. Unsupported attributes/items/expressions and
unresolved paths are reported. Ambiguous glob providers yield uncertainty.
The corpus is finite and synthetic; compiler legality is not evidence of
reference semantic completeness outside this domain.

The reference has hand-derived checks for aliases/scopes/globs, module layout,
cfg flags, heuristic propagation, named holes, M19, edition behavior, lexical
decoys, unsupported input, missing files, and a source mutation that must alter
the output. All registered data remains separate from the implementation that
will eventually be compared against it.
