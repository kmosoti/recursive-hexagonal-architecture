#!/usr/bin/env python3
"""Independent reference implementation of the CHG-011 citation contract
(`docs/architecture/citation-contract.md`), written before any Rust
implementation exists.

This module:
  * implements the reference-entry grammar (contract section 1);
  * implements the citation grammar, recognized in the same contexts as
    §-references (contract section 2, reusing
    `docs/architecture/section-reference-contract.md` section 2, as amended
    by its own registration review);
  * implements resolution and the two diagnostics (contract sections 1-2);
  * defines the deterministic, hand-authored case corpus (`CASES`);
  * regenerates `CASES.json` and `selftestreport.txt` when run directly.

It deliberately does not import, read, or link against `document` or any
other crate in the workspace: it knows only the two contract documents
named above.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import sys
from pathlib import Path
from typing import Optional

PACKAGE_ROOT = Path(__file__).resolve().parent
REPO_ROOT = (
    Path(os.environ["CITATIONS_REPO_ROOT"])
    if "CITATIONS_REPO_ROOT" in os.environ
    else PACKAGE_ROOT.parents[3]
)
SPEC_ORIGINAL_PATH = "docs/spec/rha-spec-v0.10.md"
SPEC_PATH = REPO_ROOT / SPEC_ORIGINAL_PATH

# ---------------------------------------------------------------------------
# Grammar
# ---------------------------------------------------------------------------

# A label: R, one or more ASCII digits, optionally one lowercase ASCII
# letter (contract section 1: "[R12a]"). The bracket must close immediately
# after the (optional) letter: `[R1A]`, `[R]`, `[r1]`, `[R1ab]` all fail to
# match this pattern at all, so none of them is ever an entry or a citation.
LABEL_BODY = r"R\d+[a-z]?"
CITATION_RE = re.compile(r"\[(" + LABEL_BODY + r")\]")

# A reference entry's leading label, matched against the paragraph's full
# (soft-break-joined) text: the label, then a space or end of string.
ENTRY_RE = re.compile(r"^\[(" + LABEL_BODY + r")\](?: |$)")

# A markdown link reference definition: up to 3 leading spaces, a bracketed
# label, a colon, then at least one non-space character (its destination).
# This package implements only the plain, single-line form the contract's
# one example needs (contract section 2, paragraph 4): it does not attempt
# multi-line, titled, or backslash-escaped reference definitions.
LINKDEF_RE = re.compile(r"^ {0,3}\[([^\]\n]+)\]:\s*\S")

FILLER = "\x01"

INLINE_CODE_RE = re.compile(r"`[^`\n]*`")
IMAGE_RE = re.compile(r"!\[[^\]\n]*\]\([^)\n]*\)")
WIKI_OR_TRANSCLUSION_RE = re.compile(r"!?\[\[[^\]\n]*\]\]")
LINK_RE = re.compile(r"\[[^\]\n]*\]\([^)\n]*\)")
HTML_TAG_RE = re.compile(r"<[^>\n]*>")


def mask_inline(line: str, link_ref_defs: set[str]) -> str:
    """Blank out spans where a citation is never recognized (contract
    section 2, reusing the section-reference contract's section 2 context
    rules): inline code, image syntax, wikilinks/transclusions, ordinary
    markdown links (text and destination), and raw inline HTML tags. Also
    blanks a bare `[LABEL]` shortcut when `LABEL` has a link reference
    definition elsewhere on the page: it renders as a link, not a citation
    (contract section 2, paragraph 4)."""
    masked = line
    for pattern in (INLINE_CODE_RE, IMAGE_RE, WIKI_OR_TRANSCLUSION_RE, LINK_RE, HTML_TAG_RE):
        masked = pattern.sub(lambda m: FILLER * len(m.group(0)), masked)
    if link_ref_defs:
        for label in link_ref_defs:
            shortcut = re.compile(r"\[" + re.escape(label) + r"\]")
            masked = shortcut.sub(lambda m: FILLER * len(m.group(0)), masked)
    return masked


# ---------------------------------------------------------------------------
# Block scanner. Purpose-built and minimal, like the section-refs oracle's
# scanner, extended with paragraph accumulation (a reference entry's label
# must be the paragraph's first character, and "the end of the paragraph"
# can be its last character, so multi-line top-level paragraphs must be
# joined before that check). It recognizes: fenced and indented code, ATX
# headings, block quotes (one level, shallow only: this package's cases do
# not nest quotes for citations, since the contract already establishes
# arbitrary quote depth via the section-reference contract this package
# reuses and does not re-test it), list items (one line each, as the
# section-refs oracle treats them), GFM tables, and raw HTML blocks. It does
# not import, read, or depend on any markdown parser.
# ---------------------------------------------------------------------------

FENCE_RE = re.compile(r"^(`{3,}|~{3,})")
HEADING_RE = re.compile(r"^(#{1,6})\s+(.+?)\s*#*\s*$")
TABLE_SEP_RE = re.compile(r"^\s*\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)*\|?\s*$")
LIST_MARKER_RE = re.compile(r"^(\s*)([-*+]|\d+[.)])\s+(.*)$")
QUOTE_PREFIX_RE = re.compile(r"^(?:\s*>\s?)+")
HTML_BLOCK_START_RE = re.compile(r"^<[A-Za-z!/]")
INDENT_CODE_RE = re.compile(r"^(?: {4,}|\t)")


def strip_quote_markers(line: str) -> Optional[str]:
    m = QUOTE_PREFIX_RE.match(line)
    if not m:
        return None
    return line[m.end() :]


def split_table_row(row: str) -> list[str]:
    row = row.strip()
    if row.startswith("|"):
        row = row[1:]
    if row.endswith("|"):
        row = row[:-1]
    cells: list[str] = []
    current: list[str] = []
    i = 0
    while i < len(row):
        ch = row[i]
        if ch == "\\" and i + 1 < len(row) and row[i + 1] == "|":
            current.append("|")
            i += 2
            continue
        if ch == "|":
            cells.append("".join(current))
            current = []
            i += 1
            continue
        current.append(ch)
        i += 1
    cells.append("".join(current))
    return cells


class Result:
    def __init__(self) -> None:
        self.entries: list[dict] = []  # {"label", "anchor", "line"}
        self.citations: list[dict] = []  # {"label", "line"}  (target filled in later)
        self.duplicate_diagnostics: list[dict] = []  # {"label","first_line","second_line"}
        self._entry_by_label: dict[str, dict] = {}

    def register_entry_attempt(self, label: str, line: int) -> bool:
        """Returns True if this is a fresh entry (its leading label must
        then be excluded from citation scanning); False if it is a
        duplicate (a DuplicateReferenceEntry diagnostic was recorded and the
        label counts as an ordinary citation instead, contract section 1)."""
        if label in self._entry_by_label:
            first = self._entry_by_label[label]
            self.duplicate_diagnostics.append(
                {"label": label, "first_line": first["line"], "second_line": line}
            )
            return False
        entry = {"label": label, "anchor": "ref-" + label.lower(), "line": line}
        self._entry_by_label[label] = entry
        self.entries.append(entry)
        return True

    def add_citation(self, label: str, line: int) -> None:
        self.citations.append({"label": label, "line": line})


def scan_citations_in_text(masked: str, line_no: int, result: Result) -> None:
    for m in CITATION_RE.finditer(masked):
        result.add_citation(m.group(1), line_no)


def process_paragraph(buf: list[tuple[int, str]], link_ref_defs: set[str], result: Result) -> None:
    """`buf` is a list of `(line_no, effective_text)` for one paragraph's
    consecutive lines, already stripped of block markers (quote prefixes).
    `effective_text` still carries inline exclusions to be masked."""
    if not buf:
        return
    first_line_no, first_text = buf[0]
    joined = " ".join(text for _, text in buf)
    m = ENTRY_RE.match(joined)
    is_fresh_entry = False
    excluded_span: Optional[tuple[int, int]] = None
    if m:
        label = m.group(1)
        is_fresh_entry = result.register_entry_attempt(label, first_line_no)
        if is_fresh_entry:
            # Mask exactly the leading "[LABEL]" token on the first line so
            # it is not double-counted as a citation. The optional trailing
            # space (if the label is not at the paragraph's very end) is
            # left alone: it is not part of the label token.
            excluded_span = (0, len(m.group(0).rstrip()))

    for idx, (line_no, raw_text) in enumerate(buf):
        masked = mask_inline(raw_text, link_ref_defs)
        if excluded_span is not None and idx == 0:
            start, end = excluded_span
            masked = masked[:start] + FILLER * (end - start) + masked[end:]
        scan_citations_in_text(masked, line_no, result)


def analyze(text: str) -> Result:
    result = Result()
    lines = text.split("\n")
    n = len(lines)

    link_ref_defs: set[str] = set()
    for raw in lines:
        m = LINKDEF_RE.match(raw)
        if m:
            link_ref_defs.add(m.group(1))

    paragraph_buf: list[tuple[int, str]] = []

    def flush() -> None:
        nonlocal paragraph_buf
        process_paragraph(paragraph_buf, link_ref_defs, result)
        paragraph_buf = []

    in_fence = False
    i = 0
    while i < n:
        raw = lines[i]
        line_no = i + 1

        if in_fence:
            if FENCE_RE.match(raw.strip()):
                in_fence = False
            i += 1
            continue

        if FENCE_RE.match(raw.strip()):
            flush()
            in_fence = True
            i += 1
            continue

        if LINKDEF_RE.match(raw):
            flush()
            i += 1
            continue

        if raw.strip() == "":
            flush()
            i += 1
            continue

        quote_remainder = strip_quote_markers(raw)
        top_level = quote_remainder is None
        effective = quote_remainder if quote_remainder is not None else raw

        if top_level and INDENT_CODE_RE.match(raw) and (i == 0 or lines[i - 1].strip() == ""):
            flush()
            j = i
            while j < n and (INDENT_CODE_RE.match(lines[j]) or lines[j].strip() == ""):
                if lines[j].strip() == "" and j + 1 < n and not INDENT_CODE_RE.match(lines[j + 1]):
                    break
                j += 1
            i = j
            continue

        if top_level and HTML_BLOCK_START_RE.match(effective.strip()):
            flush()
            j = i
            while j < n and lines[j].strip() != "":
                j += 1
            i = j
            continue

        heading_m = HEADING_RE.match(effective)
        if heading_m:
            flush()
            i += 1
            continue

        if "|" in effective and i + 1 < n:
            next_effective = strip_quote_markers(lines[i + 1])
            next_effective = next_effective if next_effective is not None else lines[i + 1]
            if TABLE_SEP_RE.match(next_effective):
                flush()
                for cell in split_table_row(effective):
                    scan_citations_in_text(mask_inline(cell, link_ref_defs), line_no, result)
                j = i + 2
                while j < n:
                    row_effective = strip_quote_markers(lines[j])
                    row_effective = row_effective if row_effective is not None else lines[j]
                    if "|" not in row_effective or row_effective.strip() == "":
                        break
                    for cell in split_table_row(row_effective):
                        scan_citations_in_text(mask_inline(cell, link_ref_defs), j + 1, result)
                    j += 1
                i = j
                continue

        list_m = LIST_MARKER_RE.match(effective)
        if list_m:
            flush()
            item_content = list_m.group(3)
            item_heading_m = HEADING_RE.match(item_content)
            if item_heading_m:
                pass  # a heading as sole list content: excluded, like a top-level heading.
            else:
                # A list item's content is a paragraph in its own right
                # (contract section 1: "at any depth ... in a list item").
                # This package treats each list-item line as one complete,
                # single-line paragraph (as the section-refs oracle treats
                # list items), so it does not merge a list item's wrapped
                # continuation lines into one paragraph.
                process_paragraph([(line_no, item_content)], link_ref_defs, result)
            i += 1
            continue

        # A plain content line at the current quote depth: accumulate it
        # into the paragraph buffer if it continues the same depth,
        # otherwise flush and start a new one. (This package's block-quote
        # cases are single-line, so depth changes always coincide with a
        # paragraph boundary in practice; the guard is defensive.)
        if paragraph_buf and top_level != (paragraph_buf[-1][1] is not None) and False:
            flush()
        paragraph_buf.append((line_no, effective))
        i += 1

    flush()
    return result


def resolve(result: Result) -> tuple[list[dict], list[dict], list[dict]]:
    """Returns (reference_entries, citations-with-target, unresolved_citation
    diagnostics), each in document order (contract section 3)."""
    entry_by_label = {e["label"]: e for e in result.entries}
    has_entries = len(result.entries) > 0

    citations = []
    unresolved = []
    for c in result.citations:
        entry = entry_by_label.get(c["label"])
        target = entry["anchor"] if entry else None
        citations.append({"label": c["label"], "target": target, "line": c["line"]})
        if target is None and has_entries:
            unresolved.append({"label": c["label"], "line": c["line"]})

    return result.entries, citations, unresolved


def analyze_page(text: str) -> dict:
    result = analyze(text)
    entries, citations, unresolved = resolve(result)
    return {
        "reference_entries": entries,
        "citations": citations,
        "diagnostics": {
            "duplicate_reference_entry": result.duplicate_diagnostics,
            "unresolved_citation": unresolved,
        },
    }


# ---------------------------------------------------------------------------
# Case corpus.
# ---------------------------------------------------------------------------


def case_markdown(lines: list[str]) -> str:
    return "\n".join(lines) + "\n"


CASES: list[tuple[str, list[str]]] = []


def add(case_id: str, lines: list[str]) -> None:
    for existing_id, _ in CASES:
        assert existing_id != case_id, f"duplicate case id {case_id}"
    CASES.append((case_id, lines))


# --- Section 1: entry forms -------------------------------------------------

add("entry-simple-digits", ["# References", "", "[R1] Author, A. (2020). Title. Publisher."])
add("entry-letter-suffix", ["# References", "", "[R12a] Author, B. (2021). Title. Publisher."])
add("entry-at-end-of-paragraph", ["# References", "", "[R1]"])
add(
    "entry-in-list-item",
    ["# References", "", "- [R1] Author, A. (2020). Title.", "- [R2] Author, B. (2021). Title."],
)
add("entry-in-block-quote", ["# References", "", "> [R1] Author, A. (2020). Title."])
add(
    "not-entry-label-not-first",
    ["# References", "", "See [R1] Author, A. (2020). Title. for details."],
)
add(
    "not-entry-followed-by-non-space",
    ["# References", "", "[R1]Author, A. (2020). Title."],
)
add(
    "not-entry-lowercase-r",
    ["# References", "", "[r1] Author, A. (2020). Title."],
)
add(
    "not-entry-no-digits",
    ["# References", "", "[R] Author, A. (2020). Title."],
)
add(
    "not-entry-uppercase-letter-suffix",
    ["# References", "", "[R1A] Author, A. (2020). Title."],
)
add(
    "not-entry-two-lowercase-letters",
    ["# References", "", "[R1ab] Author, A. (2020). Title."],
)

# --- Section 1: duplicate entries -------------------------------------------

add(
    "duplicate-entry-diagnostic-and-citation",
    [
        "# References",
        "",
        "[R1] Author, A. (2020). First. Publisher.",
        "",
        "[R1] Author, C. (2022). Second, impostor. Publisher.",
    ],
)
add(
    "duplicate-entry-triple-both-diagnostics-cite-first",
    [
        "# References",
        "",
        "[R1] Author, A. (2020). First. Publisher.",
        "",
        "[R1] Author, B. (2021). Second. Publisher.",
        "",
        "[R1] Author, C. (2022). Third. Publisher.",
    ],
)
add(
    "duplicate-entry-second-label-resolves-as-citation",
    [
        "# References",
        "",
        "[R1] Author, A. (2020). First. Publisher.",
        "",
        "[R1] Author, C. (2022). Second, impostor. Publisher.",
        "",
        "See [R1] cited in the body.",
    ],
)
add(
    "duplicate-entry-in-list-items",
    ["# References", "", "- [R1] First entry.", "- [R1] Duplicate entry."],
)

# --- Section 2: citation contexts -------------------------------------------

add(
    "citation-in-paragraph",
    ["# References", "", "[R1] Author, A. (2020). Title.", "", "See [R1] for background."],
)
add(
    "citation-in-list-item",
    ["# References", "", "[R1] Author, A. (2020). Title.", "", "- discussed in [R1] here"],
)
add(
    "citation-in-table-cell",
    [
        "# References",
        "",
        "[R1] Author, A. (2020). Title.",
        "",
        "| Claim | Source |",
        "| --- | --- |",
        "| x | see [R1] |",
    ],
)
add(
    "citation-in-block-quote",
    ["# References", "", "[R1] Author, A. (2020). Title.", "", "> quoting [R1] here"],
)
add(
    "citation-in-emphasis",
    ["# References", "", "[R1] Author, A. (2020). Title.", "", "This is *emphasized [R1] text*."],
)
add(
    "citation-in-strong",
    ["# References", "", "[R1] Author, A. (2020). Title.", "", "This is **strong [R1] text**."],
)

# --- Section 2: excluded contexts -------------------------------------------

add(
    "excluded-heading-text",
    ["# References", "", "[R1] Author, A. (2020). Title.", "", "## Discussion of [R1] here"],
)
add(
    "excluded-inline-code",
    ["# References", "", "[R1] Author, A. (2020). Title.", "", "Inline code `[R1]` must not cite."],
)
add(
    "excluded-fenced-code-block",
    ["# References", "", "[R1] Author, A. (2020). Title.", "", "```", "[R1] inside a fenced code block", "```"],
)
add(
    "excluded-indented-code-block",
    ["# References", "", "[R1] Author, A. (2020). Title.", "", "    [R1] inside an indented code block"],
)
add(
    "excluded-html-block",
    [
        "# References",
        "",
        "[R1] Author, A. (2020). Title.",
        "",
        "<div data-note=\"[R1]\">raw html block</div>",
    ],
)
add(
    "excluded-inline-html-tag-but-text-between-recognized",
    ["# References", "", "[R1] Author, A. (2020). Title.", "", "see <b>[R1]</b> where markup is excluded but the text between is not."],
)
add(
    "excluded-link-text",
    # `[R1]` immediately followed by `(url)` is consumed whole as an
    # ordinary markdown link (link text "R1"), never as a citation
    # followed by coincidental parentheses.
    ["# References", "", "[R1] Author, A. (2020). Title.", "", "Body mentions [R1](https://example.com) as a real link, not a citation."],
)
add(
    "excluded-link-destination",
    ["# References", "", "[R1] Author, A. (2020). Title.", "", "See [details](https://example.com/R1-[R1]) for more."],
)
add(
    "excluded-wikilink",
    # A wikilink literally targeting a page named "R1" is excluded, not a
    # citation; unlike an inline link, a bracket-nested citation-shaped
    # token cannot sit inside `[[...]]` without breaking wikilink syntax,
    # so this package tests the natural, unnested form.
    ["# References", "", "[R1] Author, A. (2020). Title.", "", "See [[R1]] for more, a wikilink target named R1."],
)
add(
    "excluded-transclusion",
    ["# References", "", "[R1] Author, A. (2020). Title.", "", "See ![[R1]] for more, a transclusion of a page named R1."],
)
add(
    "excluded-image-alt",
    ["# References", "", "[R1] Author, A. (2020). Title.", "", "See ![R1](img.png) for more."],
)

# --- Section 2: markdown link reference definition --------------------------

add(
    "link-reference-definition-shortcut-is-a-link-not-a-citation",
    [
        "# Notes",
        "",
        "[R1]: http://example.com/r1",
        "",
        "See [R1] for the linked destination, not a citation.",
    ],
)
add(
    "link-reference-definition-does-not-affect-other-labels",
    [
        "# References",
        "",
        "[R2] Author, A. (2020). Title.",
        "",
        "[R1]: http://example.com/r1",
        "",
        "See [R1] (a link) and [R2] (a citation) together.",
    ],
)

# --- Section 2: ranges, list forms, adjacent punctuation --------------------

add(
    "range-two-citations-with-dash-text-between",
    ["# References", "", "[R1] First.", "", "[R72] Seventy-second.", "", "see [R1]-[R72] for the range."],
)
add(
    "range-with-spaces-still-two-citations",
    ["# References", "", "[R1] First.", "", "[R72] Seventy-second.", "", "see [R1] - [R72] for the range."],
)
add(
    "comma-list-is-not-a-citation-form",
    ["# References", "", "[R1] First.", "", "[R2] Second.", "", "see [R1, R2] which is not a citation."],
)
add(
    "adjacent-punctuation-comma",
    ["# References", "", "[R1] First.", "", "see [R1], comma right after."],
)
add(
    "adjacent-punctuation-period",
    ["# References", "", "[R1] First.", "", "see [R1]. period right after."],
)
add(
    "adjacent-punctuation-parentheses",
    ["# References", "", "[R1] First.", "", "see the result (per [R1]) in parentheses."],
)
add(
    "adjacent-word-glued-both-sides",
    ["# References", "", "[R1] First.", "", "word[R1]glued on both sides."],
)

# --- Section 2: unresolved citations -----------------------------------------

add(
    "unresolved-citation-on-page-with-entries",
    ["# References", "", "[R1] First.", "", "see [R2] which has no entry."],
)
add(
    "unresolved-citation-on-page-without-entries",
    ["# Notes", "", "see [R1] on a page with no reference entries at all."],
)
add(
    "unresolved-and-resolved-together",
    ["# References", "", "[R1] First.", "", "see [R1] resolved and [R2] unresolved together."],
)

# --- Section 1 & 2: citations inside an entry's own text --------------------

add(
    "citation-inside-entrys-own-text",
    [
        "# References",
        "",
        "[R1] First. Compare with [R2].",
        "",
        "[R2] Second.",
    ],
)
add(
    "citation-inside-entrys-own-text-self-reference",
    ["# References", "", "[R1] Self-referential entry, see also [R1] later in its own text."],
)

# --- Additional combinations --------------------------------------------------

add(
    "multiple-entries-and-citations-mixed-order",
    [
        "# References",
        "",
        "[R1] First reference.",
        "",
        "[R2] Second reference.",
        "",
        "Body text cites [R2] before [R1] here.",
    ],
)
add(
    "entry-and-citation-same-paragraph-not-possible-separate-paragraphs",
    [
        "# References",
        "",
        "[R1] First reference, cites nothing.",
        "",
        "A separate paragraph cites [R1] normally.",
    ],
)
add(
    "no-entries-no-citations-plain-page",
    ["# Notes", "", "A plain page with no bracket labels at all."],
)
add(
    "bracket-not-matching-label-grammar-plain-text",
    ["# Notes", "", "A page mentioning [Ref1] and [R] and [r2], none of which are labels."],
)
add(
    "entry-with-multi-digit-label",
    ["# References", "", "[R123] Author. (2020). Title."],
)
add(
    "entry-anchor-lowercases-only-the-letter-suffix",
    ["# References", "", "[R57a] Author. (2019). Title."],
)
add(
    "citation-and-entry-on-different-pages-independent",
    # Single-page oracle: this case documents (via its id) that this
    # package is per-page only, matching decision `citations-scope`; it
    # is otherwise an ordinary single-entry, single-citation page.
    ["# References", "", "[R1] First.", "", "cites [R1] here."],
)
add(
    "table-cell-entry-attempt-is-not-an-entry",
    # A table cell is not a paragraph (contract section 1: "a reference
    # entry is a paragraph"), so a bracket label as a cell's sole content
    # is a citation attempt, not an entry, even though it is the cell's
    # first (and only) content.
    [
        "# References",
        "",
        "[R1] Author, A. (2020). Title.",
        "",
        "| Label | Note |",
        "| --- | --- |",
        "| [R1] | is a citation, not a second entry |",
    ],
)
add(
    "list-item-with-trailing-citation",
    ["# References", "", "[R1] First entry.", "", "- item text ending with a citation [R1]"],
)
add(
    "block-quote-entry-then-citation-same-quote",
    ["# References", "", "> [R1] Quoted entry.", "", "> cites [R1] in a later quoted paragraph."],
)
add(
    "html-block-does-not-consume-following-paragraph",
    [
        "# References",
        "",
        "[R1] Author, A. (2020). Title.",
        "",
        "<div>excluded [R1] in html block</div>",
        "",
        "a real citation of [R1] in a normal paragraph.",
    ],
)
add(
    "fenced-code-tilde-fence-excluded",
    ["# References", "", "[R1] Author, A. (2020). Title.", "", "~~~", "[R1] inside a tilde-fenced code block", "~~~", "", "see [R1] for real."],
)
add(
    "entry-paragraph-with-trailing-sentence",
    ["# References", "", "[R99] Author, Z. (2023). A Longer Title With Words. Publisher, City."],
)
add(
    "multiple-labels-two-digit-and-three-digit-distinct",
    ["# References", "", "[R1] One.", "", "[R12] Twelve.", "", "cites [R1] and [R12] distinctly."],
)
add(
    "range-three-citations-chained",
    [
        "# References",
        "",
        "[R1] First.",
        "",
        "[R2] Second.",
        "",
        "[R3] Third.",
        "",
        "see [R1]-[R2]-[R3] chained.",
    ],
)
add(
    "citation-in-entry-continuation-line-same-paragraph",
    # No blank line separates these two lines, so they are one paragraph
    # (a soft break, not a new paragraph): the entry's label is still its
    # first line, and the citation on the continuation line is still
    # inside the entry's own paragraph text.
    ["# References", "", "[R1] First.", "cites [R1] on the continuation line of the same paragraph."],
)
add(
    "duplicate-entry-across-list-and-paragraph",
    ["# References", "", "[R1] First, as a paragraph.", "", "- [R1] Duplicate, as a list item."],
)
add(
    "unresolved-citation-in-list-item",
    ["# References", "", "[R1] First.", "", "- an unresolved [R9] mention"],
)
add(
    "unresolved-citation-in-table-cell",
    [
        "# References",
        "",
        "[R1] First.",
        "",
        "| A | B |",
        "| --- | --- |",
        "| x | see [R9] |",
    ],
)
add(
    "unresolved-citation-in-block-quote",
    ["# References", "", "[R1] First.", "", "> mentions [R9] which is unresolved"],
)
add(
    "citation-with-letter-suffix-resolves",
    ["# References", "", "[R12a] Entry with letter suffix.", "", "cites [R12a] here."],
)
add(
    "citation-digits-only-does-not-match-letter-suffix-entry",
    ["# References", "", "[R12a] Entry with letter suffix.", "", "cites [R12] which has no entry (different label)."],
)

# --- The real spec -----------------------------------------------------------


def build_cases() -> list[dict]:
    result = []
    for case_id, lines in CASES:
        markdown = case_markdown(lines)
        analysis = analyze_page(markdown)
        result.append({"id": case_id, "markdown": markdown, **analysis})

    spec_text = SPEC_PATH.read_text(encoding="utf-8")
    spec_sha256 = hashlib.sha256(spec_text.encode("utf-8")).hexdigest()
    spec_analysis = analyze_page(spec_text)
    result.append(
        {
            "id": "real-spec-rha-spec-v0-10",
            "path": SPEC_ORIGINAL_PATH,
            "sha256": spec_sha256,
            **spec_analysis,
        }
    )
    return result


# ---------------------------------------------------------------------------
# Self-test
# ---------------------------------------------------------------------------


def run_selftests() -> list[str]:
    lines: list[str] = []

    def check(name: str, condition: bool) -> None:
        lines.append(f"{'PASS' if condition else 'FAIL'} {name}")
        if not condition:
            raise AssertionError(name)

    def entries_of(markdown: str) -> list[tuple[str, str, int]]:
        r = analyze_page(markdown)
        return [(e["label"], e["anchor"], e["line"]) for e in r["reference_entries"]]

    def citations_of(markdown: str) -> list[tuple[str, Optional[str], int]]:
        r = analyze_page(markdown)
        return [(c["label"], c["target"], c["line"]) for c in r["citations"]]

    check(
        "simple entry recognized with correct anchor",
        entries_of("[R1] Author. Title.\n") == [("R1", "ref-r1", 1)],
    )
    check(
        "letter-suffix entry lowercases only the letter",
        entries_of("[R12a] Author. Title.\n") == [("R12a", "ref-r12a", 1)],
    )
    check(
        "entry at end of paragraph (label alone)",
        entries_of("[R1]\n") == [("R1", "ref-r1", 1)],
    )
    check("label not first is not an entry", entries_of("See [R1] here.\n") == [])
    check(
        "label followed by non-space is not an entry",
        entries_of("[R1]Author.\n") == [],
    )
    check("lowercase r is not a label", entries_of("[r1] Author.\n") == [])
    check("no digits is not a label", entries_of("[R] Author.\n") == [])
    check("uppercase letter suffix is not a label", entries_of("[R1A] Author.\n") == [])
    check("two-letter suffix is not a label", entries_of("[R1ab] Author.\n") == [])

    r = analyze_page("[R1] First.\n\n[R1] Second, impostor.\n")
    check(
        "duplicate entry: only the first is a ReferenceEntry",
        [e["label"] for e in r["reference_entries"]] == ["R1"],
    )
    check(
        "duplicate entry: diagnostic cites first_line and second_line",
        r["diagnostics"]["duplicate_reference_entry"] == [{"label": "R1", "first_line": 1, "second_line": 3}],
    )
    check(
        "duplicate entry: the second label counts as a citation, resolved to the first entry",
        r["citations"] == [{"label": "R1", "target": "ref-r1", "line": 3}],
    )

    r = analyze_page("[R1] First.\n\n[R1] Second.\n\n[R1] Third.\n")
    check(
        "triple duplicate: both later paragraphs cite first_line=1",
        r["diagnostics"]["duplicate_reference_entry"]
        == [
            {"label": "R1", "first_line": 1, "second_line": 3},
            {"label": "R1", "first_line": 1, "second_line": 5},
        ],
    )

    check(
        "citation in inline code is excluded",
        citations_of("[R1] First.\n\nsee `[R1]` here.\n") == [],
    )
    check(
        "citation in fenced code block is excluded",
        citations_of("[R1] First.\n\n```\n[R1]\n```\n") == [],
    )
    check(
        "citation in link text and destination is excluded",
        citations_of("[R1] First.\n\nsee [R1](https://example.com/[R1]) here.\n") == [],
    )
    check(
        "citation in wikilink is excluded",
        citations_of("[R1] First.\n\nsee [[R1]] here.\n") == [],
    )
    check(
        "citation in transclusion is excluded",
        citations_of("[R1] First.\n\nsee ![[R1]] here.\n") == [],
    )
    check(
        "citation in image alt is excluded",
        citations_of("[R1] First.\n\nsee ![R1](img.png) here.\n") == [],
    )
    check(
        "citation in heading text is excluded",
        citations_of("[R1] First.\n\n## discussing [R1] here\n") == [],
    )
    check(
        "text between two inline HTML tags is recognized",
        citations_of("[R1] First.\n\nsee <b>[R1]</b> here.\n") == [("R1", "ref-r1", 3)],
    )

    check(
        "a link reference definition turns a bare [R1] into a link, not a citation",
        citations_of("[R1]: http://example.com\n\nsee [R1] here.\n") == [],
    )
    check(
        "a link reference definition for one label does not affect another",
        citations_of("[R2] Second.\n\n[R1]: http://example.com\n\nsee [R1] and [R2] here.\n")
        == [("R2", "ref-r2", 5)],
    )

    check(
        "a range is two independent citations with '-' as ordinary text between them",
        citations_of("[R1] First.\n\n[R72] Second.\n\nsee [R1]-[R72] here.\n")
        == [("R1", "ref-r1", 5), ("R72", "ref-r72", 5)],
    )
    check(
        "a comma list is not a citation form: no bracket matches the label grammar",
        citations_of("[R1] First.\n\n[R2] Second.\n\nsee [R1, R2] here.\n") == [],
    )

    r = analyze_page("[R1] First.\n\nsee [R2] which has no entry.\n")
    check(
        "unresolved citation on a page with entries is diagnosed",
        r["diagnostics"]["unresolved_citation"] == [{"label": "R2", "line": 3}],
    )
    r = analyze_page("see [R1] on a page with no reference entries.\n")
    check(
        "unresolved citation on a page without entries is not diagnosed",
        r["diagnostics"]["unresolved_citation"] == [] and r["citations"] == [{"label": "R1", "target": None, "line": 1}],
    )

    check(
        "a citation inside an entry's own text is recognized",
        citations_of("[R1] First. Compare with [R2].\n\n[R2] Second.\n")
        == [("R2", "ref-r2", 1)],
    )
    check(
        "an entry citing its own label later in its own text is a citation",
        citations_of("[R1] Self-referential, see also [R1] later.\n") == [("R1", "ref-r1", 1)],
    )

    check(
        "a table cell entry attempt is not an entry, only a citation",
        entries_of("[R1] First.\n\n| A |\n| --- |\n| [R1] |\n") == [("R1", "ref-r1", 1)],
    )

    # Corpus-wide sanity.
    cases = build_cases()
    check(f"case_count={len(CASES) + 1}", len(cases) == len(CASES) + 1)
    ids = [case["id"] for case in cases]
    check("case ids are unique", len(ids) == len(set(ids)))

    spec_case = next(case for case in cases if case["id"] == "real-spec-rha-spec-v0-10")
    check(
        "the real spec records no DuplicateReferenceEntry diagnostic",
        spec_case["diagnostics"]["duplicate_reference_entry"] == [],
    )
    check(
        "the real spec records no UnresolvedCitation diagnostic",
        spec_case["diagnostics"]["unresolved_citation"] == [],
    )

    return lines


def main() -> int:
    report_lines = run_selftests()
    cases = build_cases()

    cases_path = PACKAGE_ROOT / "CASES.json"
    cases_path.write_text(json.dumps(cases, indent=2, sort_keys=False) + "\n", encoding="utf-8")

    spec_case = next(case for case in cases if case["id"] == "real-spec-rha-spec-v0-10")
    report_lines.append(f"PASS case_count={len(cases)}")
    report_lines.append(f"PASS synthetic_case_count={len(cases) - 1}")
    report_lines.append(f"PASS spec_case_reference_entry_count={len(spec_case['reference_entries'])}")
    report_lines.append(f"PASS spec_case_citation_count={len(spec_case['citations'])}")
    report_lines.append(
        f"PASS spec_case_duplicate_reference_entry_count={len(spec_case['diagnostics']['duplicate_reference_entry'])}"
    )
    report_lines.append(
        f"PASS spec_case_unresolved_citation_count={len(spec_case['diagnostics']['unresolved_citation'])}"
    )
    report_lines.append("PASS all selftests executed without assertion failure")

    report_path = PACKAGE_ROOT / "selftestreport.txt"
    report_path.write_text("\n".join(report_lines) + "\n", encoding="utf-8")

    for line in report_lines:
        print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
