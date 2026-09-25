# Front matter and tags (`---`) fixture package (CHG-013, stage A)

This package is a pre-code, independent reference for
`docs/architecture/front-matter-contract.md` sections 1-3, snapshotted under
`source-snapshots/contract.md`. It contains **62 hand-authored cases**. There
is no seed: every case is deliberately constructed to exercise one rule or
boundary of the contract at a time, not generated from randomness.

`reference.py` implements:

- **Section 1 (recognition):** the opening-line rule (must be exactly `---`
  at line 1, no preceding blank line, no byte-order mark, trailing
  spaces/tabs allowed), the closing-line rule (`---` or `...`, same trailing
  whitespace allowance, first match wins), and every named "parses exactly
  as before" exclusion (leading blank line, no closer, `---` not at line 1,
  `+++`);
- **Section 2 (grammar):** blank and comment lines, key-value lines (key
  regex, first-occurrence-wins for `title`/`tags`), list items (indentation,
  attachment to "the most recent key-value line whose value was empty"),
  `title` (quote removal, empty-after-quote-removal is absent), `tags` (flow,
  block, and scalar forms; trim, drop-empty, ASCII-case-insensitive dedup
  keeping the first spelling, order of first appearance), unknown keys
  (ignored, no diagnostic), and `Diagnostic::InvalidFrontMatter { line }` for
  everything else (nested maps, unindented or tab-indented list items, a
  list item with no preceding empty-valued key);
- **Section 3, only as far as this package needs:** where body/markdown
  parsing starts (the line after the closing delimiter) and, when front
  matter supplies no title, the title fallback to the first level-1 ATX
  heading's text, else the page id's basename. This package's own minimal
  heading scanner recognizes only plain ATX headings (`^#{1,6} text$`, no
  trailing `#` run, no inline markup); see "Design decisions" below for why
  that is enough here and "Open questions" for what it does not attempt.

It deliberately does not import, read, or otherwise depend on `document`,
`pulldown-cmark`, or any other parser; it is independent of the
implementation under test. It also does not implement or test any markdown
rendering unrelated to front matter (headings elsewhere in a page, links,
citations, etc.) beyond the minimum needed to state a title-fallback or
heading-line expectation.

## What is registered

- `CASES.json`: 62 cases, each
  `{"id", "markdown", "front_matter", "title", "invalid_front_matter_lines",
  "headings"}`.
  - `front_matter` is `null`, or `{"title": string|null, "tags": [string,
    ...], "end_line": int}` (contract section 3's `FrontMatter` struct).
  - `title` is the expected `Document::title` string. Every case is parsed
    with `Source::new(RelPath::new("fixture.md"), markdown)` (matching the
    citations grader's convention), so the fallback basename is always
    `"fixture"`.
  - `invalid_front_matter_lines` is the ordered list of 1-based source lines
    that must produce `Diagnostic::InvalidFrontMatter { line }`, in the order
    they occur.
  - `headings` is the ordered list of `{"level", "text", "line"}` for every
    plain ATX heading this package's own scanner finds in the body (or, for
    the handful of "not recognized" negative cases, in the whole page) --
    the grader is expected to check `document.headings` against this list on
    `(level, text, line)` only (see "Design decisions": it does not assert
    slugs, which are an unrelated, already-covered feature).

## Self-check

Running `reference.py` executes 46 self-tests (embedded unit checks of the
grammar functions, independent of the corpus) plus 2 corpus-wide sanity
checks (case count, unique ids), then writes `CASES.json` and
`selftestreport.txt`. All 62 cases regenerate byte-identically from a clean
checkout; there is no external file dependency (unlike the citations
package's real-spec case, this contract has no "real page" corpus case,
since front matter is a new, unimplemented feature with no existing
production pages that use it).

## Reproduce from a fresh staging copy

```sh
mkdir -p target/m2
cp -R xtask/tests/corpus/front-matter target/m2/front-matter
cd target/m2/front-matter
python3 -B reference.py
```

`reference.py` is read-only unless run as `__main__`, in which case it
regenerates exactly two files in place: `CASES.json` and
`selftestreport.txt`. Compare the regenerated files against the committed
copies; they must be byte-identical. `SHA256SUMS` covers every payload file
except itself and `registration.toml`, in sorted relative-path order.

## Design decisions worth naming

- **List-item indentation requires a literal space, not any whitespace.**
  Contract section 2 says a list item is "indented by at least one space".
  This package reads "space" literally: a tab before the `-` does not
  satisfy the rule (`invalid-tab-indented-list-item-not-a-space`), matching
  the contract's own careful, separate use of "spaces or tabs" everywhere it
  means to allow both (the first-line and closing-line trailing-whitespace
  rules, the key-value value trim, the blank-line and comment-line
  definitions).
- **"The most recent key-value line" is tracked across intervening blank,
  comment, and invalid lines, and is recomputed on every key-value line
  regardless of its key.** A list item's validity and attachment depend only
  on whether the nearest preceding key-value line (of any kind, skipping
  non-key-value lines) had an empty value; a non-empty-valued key-value line
  in between un-attaches any earlier empty-valued key, even if that earlier
  key was `tags` (`tags-block-interrupted-by-another-key-second-item-invalid`,
  `invalid-line-does-not-change-most-recent-key-value-line`). This is the
  direct reading of "belongs to the most recent key-value line whose value
  was empty".
- **A later, non-winning occurrence of a recognized key does not lose its
  own structural validity.** "The first occurrence of each key wins; later
  ones are ignored" is read as "later ones are ignored **for the purpose of
  the key's collected value**", not as "later occurrences are diagnosed".
  A second `tags:` line with an empty value is a perfectly valid key-value
  line (no diagnostic), and its own subsequent list items are syntactically
  valid front-matter lines too -- they are just not collected into `tags`,
  because that second occurrence did not win
  (`tags-duplicate-key-first-wins-flow-then-block-ignored`).
- **The flow-list/scalar distinction for `tags` is syntactic: bracket
  delimiters, not comma presence.** `tags: a, b` (no brackets) is one
  literal scalar tag `"a, b"`, not two tags
  (`tags-scalar-with-comma-no-brackets-is-one-literal-tag`), because
  section 2 ties the comma-split behavior specifically to the bracketed
  flow-list form.
- **The grader is expected to compare `document.headings` only on `(level,
  text, line)`.** `Heading` also carries `slug` and `base_slug`
  (`crates/document/src/lib.rs`), which are an existing, independently
  specified and already-tested feature (slug de-duplication etc.) that this
  contract does not touch; asserting full `Heading` equality here would
  needlessly couple this grader to that unrelated logic.

## Open questions

These are left out of the corpus rather than guessed at:

1. **A comma inside a quoted flow-list element.** The flow-list form is
   defined as "split on commas, each element trimmed, quotes removed" with
   no mention of quote-aware splitting, so a case like
   `tags: [a, "b, c"]` is genuinely ambiguous between two tags (`a`, `"b, c"`
   naively split into `"b` and `c"`, which is almost certainly not intended)
   and some quote-aware split producing `a` and `b, c`. This package's own
   `_split_flow_list` does the naive split (matching the literal "split on
   commas" wording), but no corpus case exercises an internal comma inside
   quotes, since either reading is defensible and the contract does not
   choose.
2. **Whether a block-list item's content, or a scalar `tags` value, has its
   surrounding quotes removed.** Quote removal is stated only for `title`
   and, explicitly, for each flow-list element ("quotes removed as for
   title"); it is not stated for the block-list or scalar forms. This
   package treats block-list items and scalar values as quote-*not*-stripped
   (their content is used exactly as split/trimmed), but no case in the
   corpus uses a quoted block-list item or a quoted scalar value, since the
   opposite reading is equally defensible from the text.
3. **Whether a space is required immediately before the colon in a
   key-value line, or is tolerated.** The grammar states the shape as
   `key: value` and gives the key's regex as matching "at column 1", which
   this package reads as requiring the very next character to be `:` (no
   intervening space); a line like `title : x` is therefore not a key-value
   line under this reading. But the contract never states this negatively,
   and a permissive reading (trim trailing space after the key before
   matching `:`) is also plausible. This package does not implement or test
   either behavior for this specific shape; no case uses a space before a
   colon.
4. **Whether an empty list item (`- ` with nothing after the mandatory
   space, or a bare `-` with no space at all) is a valid, empty-string list
   item, or an invalid line.** Section 2's list-item bullet is written as
   "`- item`", always showing content; it never states what happens when
   there is no content. This package's grammar requires exactly one space
   after `-` followed by (possibly empty) content, so `- ` in isolation
   *would* be treated as an empty list item by this package's own code, but
   no case in the corpus exercises it, since the contract's own wording
   gives no basis for confidence in that reading over "invalid: not really
   an item".
5. **CRLF line endings.** The contract's "line" vocabulary (first line,
   later line, trailing spaces or tabs) is written with no mention of `\r`.
   This package, like the corpus, assumes `\n`-terminated lines throughout
   and does not test whether a trailing `\r` before the delimiter (e.g. a
   Windows-authored `---\r\n`) would count as part of "trailing spaces or
   tabs" (it is neither) or break recognition outright.
6. **Malformed flow-list syntax:** a `tags:` value that starts with `[` but
   has no matching closing `]` on the same line (e.g. `tags: [a, b`). The
   contract defines the flow-list form by its closed-bracket shape and gives
   no fallback for an open, unclosed one. This package's code happens to
   fall through to the scalar case for such a value (since it fails the
   `startswith("[") and endswith("]")` test), but no corpus case exercises
   this, since a fallback to "scalar" is an implementation choice, not
   something the contract states.
7. **Whether recognized-key case sensitivity is truly total, or whether some
   other casing of `title`/`tags` (e.g. `TAGS`) might be intended as an
   alias.** Section 2 writes the recognized keys in lowercase only and gives
   the general key grammar as case-permissive
   (`[A-Za-z_][A-Za-z0-9_-]*`); this package treats key matching as
   case-sensitive (only exactly `title`/`tags` are recognized) and has one
   case for `Title:` (`title-key-is-case-sensitive-capital-t-ignored`)
   showing the consequence of that reading, but does not exhaustively test
   every other casing, since the contract's silence here is the same kind of
   silence as question 3, not a distinct ambiguity worth multiplying cases
   over.

None of these seven points changes this package's coverage of the sections
the task asked for (every named boundary in sections 1 and 2 has at least
one unambiguous case); they are the places where a different, equally
literal reading of the contract's prose would produce a different oracle,
and are recorded here instead of resolved by guessing.
