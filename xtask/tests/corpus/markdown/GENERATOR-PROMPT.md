You are the ADVERSARIAL CORPUS GENERATOR for packet P-A of this repository (plan docs/plan/IMPLEMENTATION-PLAN.md §2.4 and §8.1 P-A stage 1). No product code exists yet, and you must not read or write anything under crates/. Write ONLY under `xtask/tests/corpus/markdown/`. Do not commit, push, or run cargo. Use git only read-only.

Goal: a registered corpus of small markdown "sites" that grade a future wiki checker, `rhawiki check --root <site> --format json`. Every expectation is fixed BY CONSTRUCTION (you plant each violation deliberately), not computed by a program. Aim for about 60 sites and 250 to 400 markdown files in total. Make them adversarial: stress the rules below at their edges, including Unicode, case, nested directories, code spans and fences, headings containing links and inline code, aliases, anchors, and near-misses that must NOT produce a witness. Include roughly one third clean sites with zero witnesses.

THE CONTRACT (from plan §3 and the W5 brief; apply it exactly):
- Page id: the file's path relative to the site root, `/`-separated, without the `.md` suffix, Unicode NFC-normalized, case preserved. Only `.md` files are pages.
- Two files whose ids are equal after NFC (e.g. `café.md` written in NFC and in NFD) produce ONE witness `{"kind":"duplicate_page_id","id":"<nfc id>"}`.
- Heading slug: take the heading's plain text (inline code text kept; emphasis and link markup removed, link text kept); Unicode-lowercase it; delete every character that is not a Unicode letter or digit, a space, `-` or `_`; replace each space with `-` (no collapsing). Headings in code fences/blocks are not headings.
- Duplicate slug in one page: the second and later get `-2`, `-3`, … deterministically, and EACH duplicate produces `{"kind":"duplicate_slug","page":"<page id>","slug":"<the base slug>"}` (one witness per extra occurrence).
- Wikilinks: `[[Target]]`, `[[Target|alias]]`, `[[Target#heading]]`, `[[Target#heading|alias]]`, `[[#heading]]` (same page). Wikilinks inside code spans or fenced/indented code are NOT links. Ordinary markdown links `[x](y)` are ignored by the checker.
- Resolution of Target: an exact page id match wins; otherwise the unique page whose basename (last path segment of the id) equals Target case-insensitively; if none → `{"kind":"broken_link","from":"<page id>","target":"<Target as written>"}`; if more than one → `{"kind":"ambiguous_link","from":"<page id>","target":"<Target as written>"}`.
- Anchor: when a link resolves and has `#heading`, `heading` must equal one of the target page's final slugs (after -2 suffixing) exactly; otherwise `{"kind":"missing_anchor","from":"<page id>","target":"<Target as written, empty string for [[#h]]>","heading":"<heading as written>"}`. A broken or ambiguous link produces no anchor witness.
- One witness per occurrence: the same broken link written twice on a page yields two identical witnesses.
- Avoid any case whose outcome depends on a rule NOT stated above (for example: emoji handling, HTML headings, setext vs ATX differences are fine only if both are clearly headings, front matter, `.markdown` files, symlinks). If you are unsure a case is unambiguous under the contract, leave it out.

LAYOUT (write exactly this):
- `xtask/tests/corpus/markdown/sites/<SITE-ID>/...` one directory per site, site ids `MD001`… in order.
- `xtask/tests/corpus/markdown/sites/<SITE-ID>/EXPECTED.json` — NOT a page (not `.md`): `{"site":"MD001","purpose":"<one line: what it stresses>","witnesses":[ ... ]}` with witnesses in any order; the checker is graded as a multiset, every key must match, no extra witnesses allowed.
- `xtask/tests/corpus/markdown/REGISTRATION.md` — a short human summary: counts of sites, files, witnesses by kind, clean sites, and the list of edge cases covered.

When done, print a summary: sites, files, witnesses by kind, clean sites. Do not print file contents.
