# Citations (`[Rn]`) fixture package (CHG-011)

This package is a pre-code, independent reference for
`docs/architecture/citation-contract.md`, which in turn reuses
`docs/architecture/section-reference-contract.md` section 2 (the
§-reference recognition-context rules) for where a citation is recognized.
Both contracts are snapshotted under `source-snapshots/`. It contains 68
hand-authored synthetic cases and one case drawn from the real, frozen spec
(`docs/spec/rha-spec-v0.10.md`), for 69 cases total. There is no seed: every
case is deliberately constructed to exercise one feature of the contract at
a time, not generated from randomness.

`reference.py` implements:

- the reference-entry grammar (contract section 1): a label `[R<digits>]`,
  optionally with one lowercase ASCII letter, as the very first characters
  of a paragraph's text, followed by a space or the paragraph's end; the
  anchor `ref-<label lowercase>`; first-entry-wins with a
  `DuplicateReferenceEntry` diagnostic and the later label counting as an
  ordinary citation;
- the citation grammar and its recognition contexts (contract section 2),
  which are exactly the section-reference contract's section 2 contexts:
  paragraphs, list items, table cells and block quotes, excluding headings,
  inline code, code blocks, raw HTML (with inline-tag text between tags
  still recognized, per that contract's own amendment), the text or
  destination of a markdown link, wikilinks, transclusions, and image alt
  text;
- resolution and the `UnresolvedCitation` diagnostic rule (contract
  section 2, paragraph 2);
- the markdown-link-reference-definition interaction (contract section 2,
  paragraph 4): a bare `[R1]` after a `[R1]: <url>` definition renders as a
  link, not a citation;

on top of its own minimal, purpose-built markdown block/inline scanner
(fenced and indented code, ATX headings, one level of block quote, list
items, GFM tables, inline code, raw HTML tags, markdown links, wikilinks,
transclusions, and images), extended with paragraph accumulation across
soft-wrapped lines (needed because a reference entry's label must be a
paragraph's very first and, for "the end of the paragraph" form, only
content). It does not import, read, or otherwise depend on `document`,
`pulldown-cmark`, or any other parser; it is independent of the
implementation under test.

## What is registered

- `CASES.json`: 69 cases, each `{"id", "markdown", "reference_entries",
  "citations", "diagnostics"}`, except the last
  (`real-spec-rha-spec-v0-10`), which instead carries `{"id", "path",
  "sha256", "reference_entries", "citations", "diagnostics"}` — the real
  spec's text is not inlined; the grader reads it from the given path in
  the live repository and checks it against the given digest first.
- `reference_entries` entries are `{"label", "anchor", "line"}`, in
  document order (contract section 3).
- `citations` entries are `{"label", "target", "line"}`, `target` being the
  resolved anchor string or `null`, in document order (contract section 3).
- `diagnostics` is an object with two arrays, since the contract defines two
  independent diagnostic kinds (this schema is this package's own design
  choice, not mandated by the contract, which only names the two variants):
  - `duplicate_reference_entry`: `{"label", "first_line", "second_line"}`,
    one per later paragraph that repeats an earlier label;
  - `unresolved_citation`: `{"label", "line"}`, one per recognized citation
    that did not resolve on a page with at least one reference entry.

## Self-check against the contract's own figures

The contract's section 5 records a pre-registration count for the real
spec: 168 entries, no duplicate label, and no unresolved citation label.
Running `reference.py` against the live `docs/spec/rha-spec-v0.10.md` gives:

- 168 reference entries (exact match with the contract's count),
- 411 recognized citations,
- 0 `DuplicateReferenceEntry` diagnostics (matches "no duplicate label"),
- 0 `UnresolvedCitation` diagnostics (matches "no unresolved citation
  label").

This is strong corroborating evidence, not a proof, that this reference
reads the contract the way its author intended, and that its reuse of the
already-registered and self-checked section-reference contract's context
rules (`xtask/tests/corpus/section-refs/`) is consistent for citations too.

## Reproduce from a fresh staging copy

```sh
mkdir -p target/m2
cp -R xtask/tests/corpus/citations target/m2/citations
cd target/m2/citations
CITATIONS_REPO_ROOT=/path/to/rha-m2 python3 -B reference.py
```

`reference.py` is read-only unless run as `__main__`, in which case it
regenerates exactly two files in place: `CASES.json` and
`selftestreport.txt`. It requires `CITATIONS_REPO_ROOT` to point at a
checkout containing `docs/spec/rha-spec-v0.10.md` only to build the spec
case (`docs/spec/` is not copied into this package); every synthetic case
needs no external file. Compare the regenerated `CASES.json` and
`selftestreport.txt` against the committed copies; they must be
byte-identical. `SHA256SUMS` covers every payload file except itself and
`registration.toml`, in sorted relative-path order.

## Design decisions worth naming

- **A citation's grammar has no trailing-boundary requirement.** The
  contract states a trailing-space-or-end rule only for a reference entry's
  *leading* label (section 1); section 2's citation grammar states none.
  This package therefore recognizes `[R1]` as a citation regardless of what
  follows it — glued to a following word, followed by punctuation, or at a
  paragraph's end — and has deliberate cases for each
  (`adjacent-punctuation-comma`, `adjacent-punctuation-period`,
  `adjacent-word-glued-both-sides`). This is the most direct reading of the
  contract text, not a guess at unstated behavior.
- **A range is not a distinct grammar construct for citations.** Unlike a
  §-reference range (where the range end is a bare number needing its own
  continuation grammar), each half of a citation range is already a
  complete, self-delimited `[R<label>]` token. This package therefore
  treats `[R1]-[R72]` as two ordinary, independently recognized citations
  with the literal text `-` between them as ordinary paragraph text,
  exactly as the contract states ("two citations with the text `-` between
  them"), needing no special range regex. `range-two-citations-with-dash-text-between`,
  `range-with-spaces-still-two-citations` and `range-three-citations-chained`
  exercise this reading; a comma list (`[R1, R2]`) simply never matches the
  label grammar at all (no digits directly after `R` inside a single
  bracket pair spanning the comma), so `comma-list-is-not-a-citation-form`
  needs no special handling either.
- **A duplicate entry's own leading label is not excluded from citation
  scanning.** The contract says the label of a paragraph that fails to
  become an entry (because its label duplicates an earlier entry) "counts
  as a citation" (section 1). This package's scanner therefore skips the
  "exclude the leading token" step precisely when a paragraph's entry
  attempt is a duplicate, and cases like
  `duplicate-entry-second-label-resolves-as-citation` and
  `duplicate-entry-diagnostic-and-citation` check that this label appears in
  `citations`, resolved to the *first* entry's anchor.
- **Both later paragraphs in a triple-duplicate cite the same `first_line`.**
  Section 1 says "the first entry with a given label is that label's
  entry", so every later duplicate diagnostic's `first_line` is the
  original entry's line, not the immediately preceding duplicate's line.
  `duplicate-entry-triple-both-diagnostics-cite-first` is a deliberate case
  for this reading.
- **A table cell can never be an entry, only a citation.** Section 1 says a
  reference entry "is a paragraph"; a table cell is not a paragraph. This
  package's scanner therefore never runs the entry-label check on a table
  cell, even when the cell's sole content is `[R1]` at its very start;
  `table-cell-entry-attempt-is-not-an-entry` is a deliberate case for this.
- **The markdown-link-reference-definition interaction is implemented only
  for its literal, single-line form**, `^ {0,3}\[label\]:\s*\S`, and only
  for the bare shortcut use `[label]` (not `[label][]` or `[label][other]`,
  which the contract's one example does not mention). See "Open questions"
  below for what this package deliberately leaves untested.

## Open questions

These are left out of the corpus rather than guessed at:

1. **Does a link reference definition for a label suppress that label's own
   entry-hood, if a paragraph elsewhere also starts with that exact
   label?** The contract's one example (`[R1]: http://x` making a later
   bare `[R1]` a link) never combines with an attempted entry for the same
   label on the same page. If the underlying markdown engine turns every
   occurrence of `[R1]` into a link once `R1` is a defined reference label —
   including the entry paragraph's own leading `[R1]` — the entry's
   rendering (`Node::Anchor` as the paragraph's first inline node, contract
   section 1) and this package's independent entry-detection grammar would
   disagree about what that paragraph's node tree looks like. This package
   does not construct a case combining a link reference definition with an
   attempted entry of the same label, and leaves the interaction untested.
2. **The full reference-definition grammar** (multi-line definitions, an
   optional title, backslash-escaped brackets, or `[label][]` /
   `[label][other]` reference-style links) is out of this package's scope;
   only the plain, single-line, bare-shortcut form the contract's one
   example needs is implemented and tested.
3. **Whether a tab, or whitespace other than one literal U+0020 space,**
   after the closing `]` of an entry's label counts as "a space" for the
   entry rule (section 1) is not stated by the citation contract. The
   analogous §-reference question was settled explicitly (U+0020 only) by
   that contract's own amendment, but the citation contract only reuses
   section 2 (recognition contexts) of that document, not section 1's
   answered ambiguity about its own, unrelated space rule. This package
   tests only a literal ASCII space and the paragraph-end form, and leaves
   a tab-after-label case out.
4. **Whether an entry paragraph's content, when the entry is inside a list
   item or a block quote, is wrapped in an explicit `Node::Paragraph`, or
   represented as the list item's/block quote's own `Vec<Node>` directly**
   (both are plausible renderings of "the paragraph can be at any depth: …
   in a list item, or in a block quote", contract section 1). The core
   grader (`crates/document/tests/citations.rs`) is written to accept
   either shape: it checks that a `Node::Anchor` is the first element of
   whichever `Vec<Node>` directly contains it, without asserting that this
   vector is specifically a `Node::Paragraph`'s children.
