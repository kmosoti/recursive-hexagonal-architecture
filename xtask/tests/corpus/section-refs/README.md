# Section-reference (§n.n) fixture package (CHG-010)

This package is a pre-code, independent reference for
`docs/architecture/section-reference-contract.md`, as amended before
registration by commit `a3ca8de` ("Settle five open questions in the
§-reference contract before registration"; contract sha256
`e86c980491bf1116dfbd008cfbc460bf8254786189b06bade1655a9c95071749`, see its
"Amendment before registration" section). It contains 83 hand-authored
synthetic cases and one case drawn from the real, frozen spec
(`docs/spec/rha-spec-v0.10.md`), for 84 cases total. There is no seed: every
case is deliberately constructed to exercise one feature of the contract at
a time, not generated from randomness.

`reference.py` implements:

- the heading-number grammar (contract section 1), applied to a heading's
  text after its leading `#`s/space and any ATX closing `#` sequence are
  stripped, wherever the heading is nested (top level, inside a block quote
  at any depth, or inside a list item);
- the reference grammar and range-continuation grammar (contract section 2),
  including the U+0020-only space/whitespace rule and the line-break
  boundary;
- resolution and the `UnresolvedSectionRef` diagnostic rule (contract
  section 3);

on top of its own minimal, purpose-built markdown block/inline scanner
(fenced and indented code, ATX headings, block quotes of any depth, list
items, GFM tables, inline code, raw HTML tags, markdown links, wikilinks,
transclusions, and images). It does not import, read, or otherwise depend on
`document`, `pulldown-cmark`, or any other parser; it is independent of the
implementation under test.

## What is registered

- `CASES.json`: 84 cases, each `{"id", "markdown", "refs", "diagnostics"}`,
  except the last (`real-spec-rha-spec-v0-10`), which instead carries
  `{"id", "path", "sha256", "refs", "diagnostics"}` — the real spec's text is
  not inlined; the grader reads it from the given path in the live
  repository and checks it against the given digest first.
- `refs` entries are `{"text", "number", "target_heading_line", "line"}`.
  `target_heading_line` is the registered 1-based source line of the target
  heading, not a slug: the grader maps that line to `Document::headings` to
  get the expected slug, so this package never re-implements `slugify`.
  `line` is the line of the reference's first character (contract section 4).
- `diagnostics` entries are `{"number", "line"}`, one per recognized
  reference that did not resolve on a page with at least one numbered
  heading.

## Self-check against the contract's own figures

The contract's section 5 records a pre-registration count for the real
spec: 163 numbered headings, no unresolved number, and 10 `§` characters in
contexts that are not references (1 in a heading, 9 in fenced code).
Running `reference.py` against the live `docs/spec/rha-spec-v0.10.md` gives:

- 163 numbered headings (exact match),
- 366 recognized references, 0 `UnresolvedSectionRef` diagnostics (matches
  the contract's "no unresolved number" claim),
- 9 `§` characters not part of any recognized reference, all inside fenced
  code blocks (out of 383 raw `§` characters in the file). The contract's
  separately noted "1 in a heading" is the same single occurrence, on the
  code-comment line `# delta in §11.8 denotes...` (spec line 2070): it
  starts with `#` and so is picked up as a heading by a naive line-prefix
  count, but it is inside a fenced code block (a shell/tree example) and
  this reference correctly does not treat it as a heading or a reference.
  This is a minor discrepancy in the contract's own informally-described
  count, not in this reference's behavior; the contract explicitly labels
  that count as a "pre-registration count", not a specification requirement
  this package must reproduce exactly.

This is strong corroborating evidence, not a proof, that this reference
reads the contract's grammar the way its author intended.

## Reproduce from a fresh staging copy

```sh
mkdir -p target/m2
cp -R xtask/tests/corpus/section-refs target/m2/section-refs
cd target/m2/section-refs
SECTION_REFS_REPO_ROOT=/path/to/rha-m2 python3 -B reference.py
```

`reference.py` is read-only unless run as `__main__`, in which case it
regenerates exactly two files in place: `CASES.json` and
`selftestreport.txt`. It requires `SECTION_REFS_REPO_ROOT` to point at a
checkout containing `docs/spec/rha-spec-v0.10.md` only to build the spec
case (`docs/spec/` is not copied into this package); every synthetic case
needs no external file. Compare the regenerated `CASES.json` and
`selftestreport.txt` against the committed copies; they must be
byte-identical. `SHA256SUMS` covers every payload file except itself and
`registration.toml`, in sorted relative-path order.

## Design decisions worth naming

- **Both ends of a range are independently resolved and independently
  diagnosed.** The contract states, in section 2, that a range
  continuation "makes that number a second reference", and section 3's
  resolution and diagnostic rules are stated generally for "a reference".
  This package reads those together: each of a range's two `SectionRef`
  entries is resolved and diagnosed on its own, exactly as a lone reference
  would be (see `range-end-unresolved`, `range-end-double-sign-rejected`,
  `range-end-letter-lookahead-rejected`). This is the most direct reading
  of the contract text, not a guess at unstated behavior, so cases exercise
  it rather than being left out.
- **A range end's optional `§` is exactly zero or one sign, never two.**
  Contract section 2 says the range continuation carries "an optional §"
  (singular). Applying the reference grammar's number pattern literally to
  a candidate range end of `§§5` fails: after optionally consuming one `§`,
  a number is required immediately, and `§` cannot start a number, so the
  whole range-continuation match fails at that position, backtracks, and
  still fails. The dash and its right-hand side are therefore left as
  ordinary text at that point, and the main scan resumes and finds `§§5` as
  its own, independent reference. `range-end-double-sign-rejected` exercises
  this literal reading.

## Open questions

None remaining. The five questions raised during registration review were
settled in the contract itself (commit `a3ca8de`, "Amendment before
registration") before this package was finalized, and this package now has
a deliberate case for each:

1. **Nested headings.** Every heading counts wherever it is nested: see
   `heading-inside-block-quote-counts`, `heading-inside-list-item-counts`,
   `heading-inside-nested-block-quote-counts`.
2. **Inline HTML.** The characters of an inline HTML tag are raw HTML and
   excluded; text between two inline tags is ordinary, recognized text:
   see `inline-html-tag-text-between-tags-is-recognized`
   (`<b>§7</b>` on a numbered page resolves).
3. **Comma list after a range.** `§§3–7, 9` gives references `3` and `7`;
   `9` is not a reference: see `range-comma-after-range-only-first-two`.
4. **The one optional space, and range whitespace, is U+0020 only,** and
   neither ever crosses a line break; a reference's `line` is the line of
   its first character: see `sign-tab-not-the-allowed-space` (a tab is not
   the allowed space), `sign-followed-by-line-break-not-a-reference` (`§`
   at the end of one line, `4.1` on the next, is not a reference), and
   `range-split-across-line-break-is-not-a-range` (`§3–` at the end of a
   line, `5` on the next, yields only `§3`).
5. Same rule as 4, restated for the range continuation specifically: a tab
   is not the allowed space there either (the range-continuation cases
   above already exercise U+0020-only spacing; `range-ascii-hyphen` and
   `range-spaces-around-dash`, from the original corpus, use plain ASCII
   spaces around the dash, never tabs).
