# Module checking contract (P-C, before extractor code)

The normative sources are spec §§3.3–3.5, 4.1, 6.13 and 9.10, plan W7, and the immutable module cases in `xtask/tests/corpus/manifest.toml`. This document fixes representation and diagnostic choices, not new structural laws. Task decision `module-diagnostics` records why the cycle cases need a non-duplicative diagnostic.

## Extraction and ownership

Traverse legal Rust module sources, including inline modules, conventional files and `#[path]`. Identify paths in imports (including nested/function imports), types, expressions and impl headers. Paths inside invocation arguments are heuristic and carry that flag. Do not expand macro definitions or `include!`; record those holes. Test-only edges are listed separately and do not enter the production graph. Unresolved internal paths are limitations, never an implicit proof of no dependency.

Resolve crate/self/super/uniform module names and local import bindings. Preserve the named facade: root `pub use ordering::score` followed by a child calling `crate::score` is an upward root reference, as M19 registers; do not replace it with the hidden ordering edge. External crate paths are outside this graph. Find module ownership by nearest declared component ancestor and classify item paths using the discovered module tree, not a fixed segment count. A facade type's associated item is not an internal module.

A declared composite's missing/malformed rules or source is a configuration/tool error with nonzero exit. A crate without a composite declaration has no module-check obligation; it must not receive a fabricated evaluated pass. The real site's declared check must actually run and pass.

## Findings and deterministic witnesses

- First classify containment. A child reference to ancestor glue/items or to an ancestor's sibling is D2 (`modules.child_to_parent_private`), even if public. The `depth` diagnostic is the number of canonical component-path segments, including the crate root as depth one; M05 `x::ordering::scoring` therefore has depth three. Such an edge is not a direct-sibling edge; `[allow]` cannot authorize it. Downward root/glue references and paths staying within one component are legitimate.
- Direct-sibling edges form the complete observed component graph. Compute strongly connected components. Emit one `modules.cycle` per cyclic component set with a deterministic closed path: lexicographically least component start, then a lexicographically least shortest return path. Any heuristic edge in that witnessed cycle marks its extraction heuristic.
- An undeclared edge inside that same cyclic set is retained as `undeclared_edges` and `subsumed_rules` on the cycle finding instead of another finding. This does not assert D4 holds; it avoids reporting the same cyclic structure twice. An undeclared edge outside a cyclic set remains `modules.undeclared_dependency`.
- A path below another component's facade is independently `modules.foreign_internal`; a cycle does not hide that separate consequence. The registered fixtures must isolate their intended violation under these rules; no expectation is silently changed.
- Findings retain the exact top-level witness fields the manifest grades (`rule`, `from`, `to`, closed `path`, depth, heuristic `extraction` where applicable), plus source locations. Module findings enter the architecture report's top-level `findings` and its summary and exit status. Per-crate `module_checks` records evaluated outcomes and limits. M11's excluded test edge appears in top-level `test_edges` as a note.

The registered headline cases are the enforcement measure. Independent seeded graphs and their reference extractor supplement it. A missed seeded violation or unexplained alarm fails the run and downgrades the affected enforcement-map cell; no maturity proposal outruns the observed corpus.
