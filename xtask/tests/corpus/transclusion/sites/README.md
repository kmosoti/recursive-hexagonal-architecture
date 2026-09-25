# Transclusion site fixture package

This is an independent pre-code fixture/reference package for the committed
transclusion contract. `reference.py` is a deterministic fixture author and
validator, not a complete Markdown, graph, or renderer reference. It makes no
target-tool calls and does not read a live implementation or oracle.

The package contains exactly twelve flat `site` rows, TX01 through TX12. The
rows cover whole-page expansion, structural exclusions, section bounds and
shells, nested embeds, resolution diagnostics, cycles, repeated imports and
navigation rebasing, and origin-aware paths. `expected_documents` covers every
source in every row. `expected_pages` records only the selected page-model
observations; it does not claim full AST equivalence.

The normal command is read-only self-test:

```sh
python3 -B reference.py
```

To reproduce in a fresh staging copy, first copy this committed package to a
fresh `target/m2` staging location. From the copied package directory, request
one fresh direct child directory and run:

```sh
python3 -B reference.py --reproduce-to reproduced
```

The copied script recreates all nine package files in `reproduced/` from the
copied frozen package inputs and performs a byte-for-byte comparison. Compare
the complete child directory if desired, then remove only that owned
`reproduced/` directory. Do not run reproduction into the committed package
data. The script rejects absolute, nested, outside, package-root, non-fresh,
and non-empty destinations.

`selector_queries` is TX05's fixed section-selection table: it records the
query, selected and excluded heading slugs, preserved wrapper, and ordered-list
start. `expected_html_contains` is TX12's named HTML rewrite control: each
core `.md` href key maps to the `.html` marker expected from the existing HTML
rewrite layer. No HTML renderer is run by this package.

`SHA256SUMS` covers every payload file in sorted relative-path order, excluding
`SHA256SUMS` and `registration.toml`. The source snapshots are frozen bytes;
registration metadata points at those committed snapshot paths and does not
require equality with future live source files.
