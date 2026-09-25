# JSON renderer corpus

This package is an independent source-model fixture set and Python reference
projector for JSON Renderer Contract v1. It does not use a Rust encoder or
claim that an adapter implementation passes.

- Fixed seed: `8072026` via `random.Random`
- Seeded random project cases: `64`
- Total cases: `79`
- Case kinds: `{"cli_check": 1, "cli_html": 2, "cli_json": 1, "collision": 1, "contract": 1, "duplicate": 2, "project": 70, "reconcile": 1}`

Coverage includes every source `Node` variant; all four table alignments; all
five callout kinds plus null; null and non-null code-block languages and list
starts; nested containers; resolved and unresolved links; empty vectors;
Unicode, Unicode whitespace, raw HTML text, literal query/backslash/quote
hrefs; independent TOC data; source ordering and reversed input; adjacent
inline fragments; distinct block, list, and table-cell boundaries; and index
clock independence.

The explicit flow cases cover duplicate IDs in two orderings, fresh-renderer
reconciliation with deletion, pre-write page/asset collision, the owner
contract controls, JSON CLI selection, HTML default and explicit selection,
and unchanged JSON check output. Their expected fields are stored alongside
their source inputs in `CASES.json`.

## Reference commands

`python3 reference.py` runs read-only self-tests. From the repository root,
copy the committed package to a fresh, owned staging directory before
reproducing:

`cp -R xtask/tests/corpus/json-renderer target/m2/json-renderer-reproduction`

`python3 target/m2/json-renderer-reproduction/reference.py --reproduce-to target/m2/json-renderer-reproduction/reproduced`

The staging destination must be fresh. The reproduction writes only under the
staged package, and its `reproduced/` subdirectory may be removed afterward.
The `source-snapshots/` directory freezes the contract, model vocabulary, and
all three durable prompt archives. Reproduction reads those snapshots, never
future live source files, and copies the snapshots into the explicit
reproduction subdirectory. Self-tests from the committed script are
read-only.

JSON expected values are decoded objects in the case stream; deterministic
pretty UTF-8 output with one final newline is a separate renderer obligation.
