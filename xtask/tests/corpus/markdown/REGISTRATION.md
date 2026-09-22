# Markdown corpus registration — P-A stage 1

Each `sites/<SITE-ID>` directory is an independent `--root` for `rhawiki check --root <site> --format json`. Its `EXPECTED.json` records the purpose and fixed witness multiset. Compare all keys exactly, preserve repeated identical entries, and reject extras.

Expectations are assigned by construction from the explicit corpus assignment. Neither the product checker nor a generated reference implementation was run. Counts below are an inventory of stored expectations, not observed checker results. Page IDs use NFC. Heading slugs follow the explicit character-filter rules without an added NFC step (MD051). IDs are construction sample indices; no random generator is involved.

| Inventory | Count |
| --- | ---: |
| Sites (`MD001`–`MD060`) | 60 |
| Markdown page files (5 per site) | 300 |
| `EXPECTED.json` manifests | 60 |
| Clean sites (`MD001`–`MD020`) | 20 |
| Sites with violations | 40 |
| `broken_link` witnesses | 27 |
| `ambiguous_link` witnesses | 8 |
| `missing_anchor` witnesses | 31 |
| `duplicate_slug` witnesses | 26 |
| `duplicate_page_id` witnesses | 7 |
| Total expected witnesses | 99 |
| Registration Markdown file | 1 |
| Total Markdown files | 301 |
| Total files | 361 |

The corpus covers these edge cases:

- Exact root and nested identities, repeated basenames, exact-before-fallback precedence, preserved case, and case-insensitive unique basename fallback (`MD001`–`MD005`, `MD020`, `MD025`–`MD028`).
- Unicode Latin, Greek, Cyrillic, and CJK letters and digits; single NFD filenames with NFC targets and source IDs; canonical collisions in filenames, directories, and multiple segments; Angstrom sign and dakuten; and case-distinct near misses (`MD006`–`MD007`, `MD012`, `MD040`, `MD050`–`MD052`, `MD055`–`MD060`).
- Aliases and same-page anchors; exact case, accent, and suffix matching; raw target and heading spelling; and broken or ambiguous links suppressing anchor witnesses (`MD008`–`MD009`, `MD023`, `MD028`–`MD033`, `MD040`, `MD054`).
- Inline code, ordinary links, emphasis in headings, punctuation deletion, retained underscores and hyphens, non-collapsed spaces, and all six ATX levels (`MD010`–`MD012`, `MD019`, `MD039`, `MD043`–`MD045`, `MD051`–`MD053`).
- Single- and double-backtick code spans, including wiki text in headings; backtick and tilde fences; indented code; parsing resumption after code; and ignored ordinary Markdown links (`MD013`–`MD017`, `MD034`–`MD038`, `MD046`–`MD047`).
- Per-occurrence identical witnesses, duplicate bases across heading levels, independent per-base and per-page counters, final `-2`/`-3`/`-4` anchors, and valid near misses across pages (`MD018`, `MD022`, `MD028`–`MD029`, `MD031`, `MD033`–`MD034`, `MD041`–`MD054`).
- Only `.md` files are pages; `.md` suffixes in targets are literal, and `EXPECTED.json` is not a page (`MD024`).

Scope excludes front matter, HTML headings, symlinks, malformed wiki syntax, natural heading or suffix collisions, and Unicode case-folding ambiguities. Duplicates of page IDs are isolated from link processing. These are unspecified rules excluded from the corpus, not new behavior.
