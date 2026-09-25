#!/usr/bin/env python3
"""Independent reference implementation of the CHG-010 section-reference
contract (`docs/architecture/section-reference-contract.md`), written before
any Rust implementation exists.

This module:
  * implements the contract's heading-number grammar (contract section 1)
    and reference grammar (contract section 2-3) directly on markdown
    source text, using its own minimal block scanner;
  * defines the deterministic, hand-authored case corpus (`CASES`);
  * regenerates `CASES.json` and `selftestreport.txt` when run directly.

It deliberately does not import, read, or link against `document` or any
other crate in the workspace: it knows only the contract text.
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Optional

PACKAGE_ROOT = Path(__file__).resolve().parent
import os
REPO_ROOT = Path(os.environ["SECTION_REFS_REPO_ROOT"]) if "SECTION_REFS_REPO_ROOT" in os.environ else PACKAGE_ROOT.parents[3]
SPEC_ORIGINAL_PATH = "docs/spec/rha-spec-v0.10.md"
SPEC_PATH = REPO_ROOT / SPEC_ORIGINAL_PATH

# ---------------------------------------------------------------------------
# Grammar (contract sections 1-3)
# ---------------------------------------------------------------------------

# A "number" is N(.N)* (one or more ASCII digits, dot-separated groups) or
# L(.N)* (one ASCII uppercase letter, dot-separated digit groups), where the
# letter form only counts when the character right after the letter is not
# itself an ASCII letter (contract section 2, paragraph 2).
NUMBER_PATTERN = r"\d+(?:\.\d+)*|[A-Z](?![A-Za-z])(?:\.\d+)*"

# A reference: section-sign(s), at most one literal space, then a number.
REF_RE = re.compile(r"§(§)?( ?)(" + NUMBER_PATTERN + r")")

# A range continuation directly after a matched number: optional
# whitespace, a dash (hyphen or en dash), optional whitespace, an optional
# single section sign, then a number (contract section 2, paragraph 3).
RANGE_RE = re.compile(r"[ \t]*[–\-][ \t]*(§?)(" + NUMBER_PATTERN + r")")

# Heading-number grammar (contract section 1). Applied to `Heading::text`,
# i.e. the heading line with its leading `#`s and one following space, and
# any ATX closing `#` sequence, already stripped.
NUMERIC_HEAD_RE = re.compile(r"^(\d+(?:\.\d+)*)(\.)?(?:\s|$)")
APPENDIX_HEAD_RE = re.compile(r"^Appendix\s+([A-Z])((?:\.\d+)*)(\.)?(?:\s|$)")


def heading_number(text: str) -> Optional[str]:
    """The section number of a heading's text, or None (contract section 1)."""
    m = NUMERIC_HEAD_RE.match(text)
    if m:
        return m.group(1)
    m = APPENDIX_HEAD_RE.match(text)
    if m:
        return m.group(1) + m.group(2)
    return None


# ---------------------------------------------------------------------------
# Inline exclusion (contract section 2, paragraph 1): a reference is never
# recognized inside inline code, a code block, raw HTML, the text or
# destination of a markdown link or a wikilink, a transclusion, or an
# image's alt text. Excluded spans are overwritten with a filler byte that
# can never itself start or continue a reference.
# ---------------------------------------------------------------------------

FILLER = "\x01"

INLINE_CODE_RE = re.compile(r"`[^`\n]*`")
IMAGE_RE = re.compile(r"!\[[^\]\n]*\]\([^)\n]*\)")
WIKI_OR_TRANSCLUSION_RE = re.compile(r"!?\[\[[^\]\n]*\]\]")
LINK_RE = re.compile(r"\[[^\]\n]*\]\([^)\n]*\)")
HTML_TAG_RE = re.compile(r"<[^>\n]*>")


def mask_inline(line: str) -> str:
    masked = line
    for pattern in (INLINE_CODE_RE, IMAGE_RE, WIKI_OR_TRANSCLUSION_RE, LINK_RE, HTML_TAG_RE):
        masked = pattern.sub(lambda m: FILLER * len(m.group(0)), masked)
    return masked


# ---------------------------------------------------------------------------
# Block scanner. This is intentionally a minimal, purpose-built scanner: it
# recognizes only the block constructs the contract and the corpus exercise
# (fenced code, indented code, ATX headings, block quotes of any depth,
# list items, GFM tables, and raw-HTML blocks), not the whole of CommonMark.
# It is independent of, and does not consult, any markdown library.
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
    current = []
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


class Analysis:
    def __init__(self) -> None:
        self.refs: list[dict] = []
        self.headings: list[dict] = []  # {"line": int, "text": str, "number": str|None}

    def add_ref(self, text: str, number: str, line: int) -> None:
        self.refs.append({"text": text, "number": number, "line": line})

    def add_heading(self, line: int, text: str) -> None:
        self.headings.append({"line": line, "text": text, "number": heading_number(text)})


def scan_content(content: str, line_no: int, analysis: Analysis) -> None:
    masked = mask_inline(content)
    pos = 0
    length = len(masked)
    while pos < length:
        m = REF_RE.search(masked, pos)
        if not m:
            break
        analysis.add_ref(m.group(0), m.group(3), line_no)
        pos = m.end()
        rm = RANGE_RE.match(masked, pos)
        if rm:
            sign = rm.group(1)
            number = rm.group(2)
            analysis.add_ref((sign or "") + number, number, line_no)
            pos = rm.end()


def analyze_blocks(text: str, analysis: Analysis) -> None:
    lines = text.split("\n")
    n = len(lines)
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
            in_fence = True
            i += 1
            continue

        if raw.strip() == "":
            i += 1
            continue

        quote_remainder = strip_quote_markers(raw)
        top_level = quote_remainder is None
        effective = quote_remainder if quote_remainder is not None else raw

        if top_level and INDENT_CODE_RE.match(raw) and (i == 0 or lines[i - 1].strip() == ""):
            j = i
            while j < n and (INDENT_CODE_RE.match(lines[j]) or lines[j].strip() == ""):
                if lines[j].strip() == "" and j + 1 < n and not INDENT_CODE_RE.match(lines[j + 1]):
                    break
                j += 1
            i = j
            continue

        if top_level and HTML_BLOCK_START_RE.match(effective.strip()):
            j = i
            while j < n and lines[j].strip() != "":
                j += 1
            i = j
            continue

        # Every heading counts wherever it is nested: a heading inside a
        # block quote has a section number by the same rule as a top-level
        # heading (contract section 1, amended). A heading inside a list
        # item is handled in the list-item branch below.
        heading_m = HEADING_RE.match(effective)
        if heading_m:
            analysis.add_heading(line_no, heading_m.group(2))
            i += 1
            continue

        if "|" in effective and i + 1 < n:
            next_effective = strip_quote_markers(lines[i + 1])
            next_effective = next_effective if next_effective is not None else lines[i + 1]
            if TABLE_SEP_RE.match(next_effective):
                for cell in split_table_row(effective):
                    scan_content(cell, line_no, analysis)
                j = i + 2
                while j < n:
                    row_effective = strip_quote_markers(lines[j])
                    row_effective = row_effective if row_effective is not None else lines[j]
                    if "|" not in row_effective or row_effective.strip() == "":
                        break
                    for cell in split_table_row(row_effective):
                        scan_content(cell, j + 1, analysis)
                    j += 1
                i = j
                continue

        list_m = LIST_MARKER_RE.match(effective)
        if list_m:
            item_content = list_m.group(3)
            item_heading_m = HEADING_RE.match(item_content)
            if item_heading_m:
                analysis.add_heading(line_no, item_heading_m.group(2))
            else:
                scan_content(item_content, line_no, analysis)
            i += 1
            continue

        scan_content(effective, line_no, analysis)
        i += 1


def analyze(text: str) -> tuple[list[dict], list[dict]]:
    """Return (refs, diagnostics) exactly as the contract defines them."""
    analysis = Analysis()
    analyze_blocks(text, analysis)

    numbers: dict[str, int] = {}
    for heading in analysis.headings:
        number = heading["number"]
        if number is not None and number not in numbers:
            numbers[number] = heading["line"]

    has_numbered_heading = len(numbers) > 0

    refs = []
    diagnostics = []
    for ref in analysis.refs:
        target_line = numbers.get(ref["number"])
        refs.append(
            {
                "text": ref["text"],
                "number": ref["number"],
                "target_heading_line": target_line,
                "line": ref["line"],
            }
        )
        if target_line is None and has_numbered_heading:
            diagnostics.append({"number": ref["number"], "line": ref["line"]})

    return refs, diagnostics


# ---------------------------------------------------------------------------
# Case corpus. Each case is a standalone page. `markdown` uses `\n` joins;
# trailing newline is added uniformly by `case_markdown`.
# ---------------------------------------------------------------------------


def case_markdown(lines: list[str]) -> str:
    return "\n".join(lines) + "\n"


CASES: list[tuple[str, list[str]]] = []


def add(case_id: str, lines: list[str]) -> None:
    for existing_id, _ in CASES:
        assert existing_id != case_id, f"duplicate case id {case_id}"
    CASES.append((case_id, lines))


# --- Section 1: heading number forms -------------------------------------

add("heading-numeric-simple", ["# 1. Problem, Scope, and Design Goals", "", "See §1 above."])
add("heading-numeric-multipart", ["## 1.1 Target class", "", "See §1.1 above."])
add("heading-numeric-multipart-three", ["### 11.7.6 Acceptance", "", "See §11.7.6 above."])
add("heading-numeric-no-trailing-dot", ["## 2 Overview", "", "See §2 above."])
add("heading-numeric-end-of-heading-text", ["## 3.4", "", "See §3.4 above."])
add("heading-appendix-simple", ["# Appendix A. Mathematical Foundations", "", "See §A above."])
add("heading-appendix-multipart", ["## Appendix B.2 Definitions", "", "See §B.2 above."])
add("heading-appendix-no-trailing-dot", ["# Appendix C Overview", "", "See §C above."])
add("heading-appendix-end-of-heading-text", ["# Appendix D", "", "See §D above."])
add(
    "heading-leading-zero-distinct",
    [
        "## 01. First",
        "## 1. Second",
        "",
        "References: §01 and §1.",
    ],
)
add(
    "heading-decimal-distinct",
    [
        "## 1.10 Ten",
        "## 1.1 One",
        "",
        "References: §1.10 and §1.1.",
    ],
)
add(
    "heading-duplicate-number-first-wins",
    [
        "## 5. First Instance",
        "",
        "Some text.",
        "",
        "## 5. Second Instance",
        "",
        "See §5 above.",
    ],
)
add(
    "heading-numeric-and-appendix-together",
    [
        "## 1. Intro",
        "## Appendix A. Foo",
        "",
        "See §1 and §A.",
    ],
)

# --- Section 2: recognition contexts --------------------------------------

add("context-paragraph", ["## 4. Heading", "", "A plain paragraph mentions §4 here."])
add("context-list-item", ["## 4. Heading", "", "- an item mentioning §4 inline"])
add(
    "context-table-cell",
    [
        "## 4. Heading",
        "",
        "| A | B |",
        "| --- | --- |",
        "| x | see §4 |",
    ],
)
add("context-blockquote-shallow", ["## 4. Heading", "", "> quoting §4 here"])
add("context-blockquote-nested", ["## 4. Heading", "", "> > > deeply quoting §4 here"])
add(
    "context-not-in-heading",
    ["## 4. Heading mentions §4 inline", "", "No reference expected anywhere."],
)
add(
    "context-not-in-inline-code",
    ["## 4. Heading", "", "Inline code `§4` must not be a reference."],
)
add(
    "context-not-in-fenced-code",
    ["## 4. Heading", "", "```", "§4 inside a fenced code block", "```"],
)
add(
    "context-not-in-indented-code",
    ["## 4. Heading", "", "    §4 inside an indented code block"],
)
add(
    "context-not-in-html-block",
    ["## 4. Heading", "", "<div data-note=\"§4\">raw html block</div>"],
)
add(
    "context-not-in-link-text",
    ["## 4. Heading", "", "See [§4 details](https://example.com/x) for more."],
)
add(
    "context-not-in-link-destination",
    ["## 4. Heading", "", "See [details](https://example.com/§4) for more."],
)
add(
    "context-not-in-wikilink",
    ["## 4. Heading", "", "See [[Page§4|alias]] for more."],
)
add(
    "context-not-in-transclusion",
    ["## 4. Heading", "", "See ![[Page#§4]] for more."],
)
add(
    "context-not-in-image-alt",
    ["## 4. Heading", "", "See ![§4 alt](img.png) for more."],
)

# --- Section 2: sign forms -------------------------------------------------

add("sign-single", ["## 7. Heading", "", "See §7 above."])
add("sign-double", ["## 7. Heading", "", "See §§7 above."])
add("sign-one-space", ["## 4.1 Heading", "", "See § 4.1 above."])
add("sign-two-spaces-not-a-reference", ["## 4.1 Heading", "", "See §  4.1 above (two spaces)."])
add("sign-sentence-final-period", ["## 7. Heading", "", "see §7."])
add(
    "sign-multipart-sentence-final-period",
    ["## 7.1 Heading", "", "see §7.1."],
)
add("sign-letter-simple", ["# Appendix A. Foo", "", "see §A here."])
add("sign-letter-multipart", ["## Appendix A.1 Foo", "", "see §A.1 here."])
add("sign-letter-not-followed-by-letter", ["# Appendix A. Foo", "", "see §Also not a reference."])
add("sign-bare-alone", ["## 4. Heading", "", "A stray § with nothing after."])
add("sign-followed-by-punctuation", ["## 4. Heading", "", "A stray §, with a comma after."])
add("sign-followed-by-dash-no-leading-number", ["## 4. Heading", "", "A stray §-7 with no number first."])
add("sign-double-not-followed-by-number", ["## 4. Heading", "", "A stray §§ with nothing after."])
add("sign-lowercase-letter-not-a-number", ["## 4. Heading", "", "see §a which is lowercase."])
add(
    "sign-letter-followed-by-digit-no-dot",
    ["# Appendix A. Foo", "", "see §A1 where 1 is plain text."],
)
add(
    "sign-longest-match-distinguishes-multipart",
    ["## 7.1 One", "## 7.10 Ten", "", "compare §7.1 with §7.10 in the same sentence."],
)
add(
    "sign-two-consecutive-no-separator",
    ["## 3. Three", "## 4. Four", "", "adjacent §3§4 with no separator."],
)
add("sign-double-resolves-single-heading", ["## 4.1 Heading", "", "see §§4.1 above."])

# --- Section 2: ranges ------------------------------------------------------

add(
    "range-double-sign-en-dash",
    ["## 3. Three", "## 7. Seven", "", "see §§3–7 for the range."],
)
add(
    "range-single-sign-both-ends",
    ["## 3. Three", "## 5. Five", "", "see §3–§5 for the range."],
)
add(
    "range-multipart-en-dash",
    ["## 8.3 Eight Three", "## 8.4 Eight Four", "", "see §§8.3–8.4 for the range."],
)
add(
    "range-ascii-hyphen",
    ["## 3. Three", "## 5. Five", "", "see §3-5 for the range."],
)
add(
    "range-spaces-around-dash",
    ["## 3. Three", "## 5. Five", "", "see §3 - 5 for the range."],
)
add(
    "range-end-unresolved",
    ["## 3. Three", "", "see §3–99 where 99 does not exist."],
)
add(
    "range-comma-list-only-first",
    ["## 8.3 Eight Three", "## 8.4 Eight Four", "", "see §§8.3, 8.4 (only the first counts)."],
)
add(
    "range-and-two-separate-references",
    ["## 6. Six", "## 12.1 Twelve One", "", "see §6 and §12.1 together."],
)
add(
    "range-end-double-sign-rejected",
    ["## 3. Three", "## 5. Five", "", "see §3–§§5 where the range grammar rejects a double sign end."],
)
add(
    "range-end-letter",
    ["# Appendix A. Foo", "## Appendix B. Bar", "", "see §A–B for the range."],
)
add(
    "range-end-letter-lookahead-rejected",
    ["# Appendix A. Foo", "", "see §A–Bz where Bz is not a number."],
)

# --- Section 3: resolution and diagnostics ---------------------------------

add(
    "resolve-basic-success",
    ["## 4.1 Heading", "", "see §4.1 which resolves."],
)
add(
    "resolve-numbered-page-unresolved-diagnostic",
    ["## 4.1 Heading", "", "see §99 which does not exist."],
)
add(
    "resolve-unnumbered-page-no-diagnostic",
    ["## Introduction", "", "see §4.1 on a page with no numbered headings."],
)
add(
    "resolve-no-headings-at-all-no-diagnostic",
    ["A page with no headings mentions §4.1 here."],
)
add(
    "resolve-headings-present-none-numbered-no-diagnostic",
    ["## Introduction", "## Overview", "", "see §3 where no heading is numbered."],
)
add(
    "resolve-exact-string-not-prefix",
    [
        "## 1.1 One One",
        "## 1.10 One Ten",
        "## 11.1 Eleven One",
        "",
        "see §1.1 which must match only 1.1, not 1.10 or 11.1.",
    ],
)

# --- Multi-line paragraphs ---------------------------------------------------

add(
    "multiline-ref-on-second-line",
    [
        "## 4. Heading",
        "",
        "This paragraph spans",
        "three lines and mentions §4",
        "on the middle line here.",
    ],
)
add(
    "multiline-multiple-refs-different-lines",
    [
        "## 3. Three",
        "## 5. Five",
        "",
        "First line mentions §3 here,",
        "and the second line mentions §5 here.",
    ],
)

# --- Additional context combinations ----------------------------------------

add(
    "context-table-header-and-body-cell",
    [
        "## 3. Three",
        "## 5. Five",
        "",
        "| see §3 | B |",
        "| --- | --- |",
        "| x | see §5 |",
    ],
)
add(
    "context-blockquote-with-range",
    ["## 3. Three", "## 5. Five", "", "> see §3–5 inside a quote"],
)
add(
    "context-list-item-with-appendix-ref",
    ["# Appendix A. Foo", "## Appendix A.1 Bar", "", "- mentions §A.1 in a list item"],
)
add(
    "context-table-separator-row-not-scanned",
    [
        "## 3. Three",
        "",
        "| A | B |",
        "| --- | --- |",
        "| §3 | y |",
    ],
)
add(
    "context-duplicate-and-unrelated-ref-same-page",
    [
        "## 5. First Instance",
        "## 9. Other",
        "",
        "Some text.",
        "",
        "## 5. Second Instance",
        "",
        "see §5 and §9 together.",
    ],
)
add(
    "context-ref-immediately-before-heading",
    [
        "## 3. Three",
        "",
        "a line mentioning §3 right before a heading",
        "## 5. Five",
        "",
        "see §5 too.",
    ],
)
add(
    "context-html-block-does-not-consume-following-paragraph",
    [
        "## 4. Heading",
        "",
        "<div>excluded §4 in html block</div>",
        "",
        "a real reference to §4 in a normal paragraph.",
    ],
)
add(
    "context-fenced-code-with-tilde-fence",
    ["## 4. Heading", "", "~~~", "§4 inside a tilde-fenced code block", "~~~", "", "see §4 for real."],
)
add(
    "context-indented-code-followed-by-paragraph",
    [
        "## 4. Heading",
        "",
        "    §4 indented code, excluded",
        "",
        "a real reference to §4 afterwards.",
    ],
)
add(
    "context-not-in-link-text-and-destination-both",
    ["## 4. Heading", "", "[§4 in text](https://example.com/§4-in-dest) and a real §4 after."],
)

# --- Amendment (2026-09-24, commit a3ca8de): nested headings, inline HTML
# text, comma-after-range, and the space/line-break boundary of a reference
# and of a range continuation. ------------------------------------------------

add(
    "heading-inside-block-quote-counts",
    [
        "> ## 1. Nested In A Quote",
        ">",
        "> body text inside the quote",
        "",
        "see §1 which targets the heading nested in the block quote.",
    ],
)
add(
    "heading-inside-list-item-counts",
    [
        "- ## 1. Nested In A List Item",
        "",
        "see §1 which targets the heading nested in the list item.",
    ],
)
add(
    "heading-inside-nested-block-quote-counts",
    [
        "> > ## 2. Doubly Nested",
        "> >",
        "> > body text",
        "",
        "see §2 which targets the doubly nested heading.",
    ],
)
add(
    "inline-html-tag-text-between-tags-is-recognized",
    ["## 7. Heading", "", "see <b>§7</b> where the tag markup is excluded but the text between is not."],
)
add(
    "range-comma-after-range-only-first-two",
    [
        "## 3. Three",
        "## 7. Seven",
        "## 9. Nine",
        "",
        "see §§3–7, 9 where only 3 and 7 are references.",
    ],
)
add(
    "sign-tab-not-the-allowed-space",
    ["## 4.1 Heading", "", "see §\t4.1 where a tab is not the allowed single space."],
)
add(
    "sign-followed-by-line-break-not-a-reference",
    [
        "## 4.1 Heading",
        "",
        "see § at the end of this line",
        "4.1 continues on the next line but is not part of any reference.",
    ],
)
add(
    "range-split-across-line-break-is-not-a-range",
    [
        "## 3. Three",
        "",
        "see §3–",
        "5 is on the next line, so no range forms across the break.",
    ],
)


def build_cases() -> list[dict]:
    result = []
    for case_id, lines in CASES:
        markdown = case_markdown(lines)
        refs, diagnostics = analyze(markdown)
        result.append(
            {
                "id": case_id,
                "markdown": markdown,
                "refs": refs,
                "diagnostics": diagnostics,
            }
        )

    spec_text = SPEC_PATH.read_text(encoding="utf-8")
    spec_sha256 = hashlib.sha256(spec_text.encode("utf-8")).hexdigest()
    spec_refs, spec_diagnostics = analyze(spec_text)
    result.append(
        {
            "id": "real-spec-rha-spec-v0-10",
            "path": SPEC_ORIGINAL_PATH,
            "sha256": spec_sha256,
            "refs": spec_refs,
            "diagnostics": spec_diagnostics,
        }
    )
    return result


# ---------------------------------------------------------------------------
# Self-test: hand-verified controls, asserted directly against the reference
# functions (not merely regenerated), plus corpus-wide sanity checks.
# ---------------------------------------------------------------------------


def run_selftests() -> list[str]:
    lines: list[str] = []

    def check(name: str, condition: bool) -> None:
        lines.append(f"{'PASS' if condition else 'FAIL'} {name}")
        if not condition:
            raise AssertionError(name)

    # Heading-number grammar controls.
    check("heading_number('1. Problem') == '1'", heading_number("1. Problem, Scope") == "1")
    check("heading_number('1.1 Target') == '1.1'", heading_number("1.1 Target class") == "1.1")
    check(
        "heading_number('11.7.6 Acceptance') == '11.7.6'",
        heading_number("11.7.6 Acceptance") == "11.7.6",
    )
    check("heading_number('2 Overview') == '2'", heading_number("2 Overview") == "2")
    check("heading_number('3.4') == '3.4'", heading_number("3.4") == "3.4")
    check(
        "heading_number('Appendix A. Math') == 'A'",
        heading_number("Appendix A. Mathematical Foundations") == "A",
    )
    check(
        "heading_number('Appendix B.2 Definitions') == 'B.2'",
        heading_number("Appendix B.2 Definitions") == "B.2",
    )
    check("heading_number('Appendix C Overview') == 'C'", heading_number("Appendix C Overview") == "C")
    check("heading_number('Appendix D') == 'D'", heading_number("Appendix D") == "D")
    check("heading_number('Appendix Also') is None", heading_number("Appendix Also") is None)
    check("heading_number('01. First') == '01'", heading_number("01. First") == "01")
    check("heading_number('1.10 Ten') == '1.10'", heading_number("1.10 Ten") == "1.10")
    check("heading_number('Introduction') is None", heading_number("Introduction") is None)

    # Reference-recognition controls, exercised end-to-end through analyze().
    def refs_of(markdown: str) -> list[tuple[str, str]]:
        refs, _ = analyze(markdown)
        return [(r["text"], r["number"]) for r in refs]

    check(
        "'see §7.' yields number 7 with the period excluded",
        refs_of("## 7. Heading\n\nsee §7.\n") == [("§7", "7")],
    )
    check(
        "two spaces after § is not a reference",
        refs_of("## 4.1 Heading\n\nsee §  4.1 here.\n") == [],
    )
    check(
        "'§Also' is not a reference",
        refs_of("# Appendix A. Foo\n\nsee §Also here.\n") == [],
    )
    check(
        "'§§3–7' yields two references: §§3 and 7",
        refs_of("## 3. Three\n## 7. Seven\n\nsee §§3–7 here.\n") == [("§§3", "3"), ("7", "7")],
    )
    check(
        "'§3–§5' yields two references: §3 and §5",
        refs_of("## 3. Three\n## 5. Five\n\nsee §3–§5 here.\n") == [("§3", "3"), ("§5", "5")],
    )
    check(
        "'§§8.3, 8.4' yields only the first reference",
        refs_of("## 8.3 A\n## 8.4 B\n\nsee §§8.3, 8.4 here.\n") == [("§§8.3", "8.3")],
    )
    check(
        "'§6 and §12.1' yields two independent references",
        refs_of("## 6. Six\n## 12.1 Twelve One\n\nsee §6 and §12.1 here.\n")
        == [("§6", "6"), ("§12.1", "12.1")],
    )
    check(
        "inline code is excluded",
        refs_of("## 4. Heading\n\nsee `§4` here.\n") == [],
    )
    check(
        "fenced code block is excluded",
        refs_of("## 4. Heading\n\n```\n§4\n```\n") == [],
    )
    check(
        "link text and destination are excluded",
        refs_of("## 4. Heading\n\nsee [§4](https://example.com/§4) here.\n") == [],
    )
    check(
        "wikilink is excluded",
        refs_of("## 4. Heading\n\nsee [[Page§4]] here.\n") == [],
    )
    check(
        "transclusion is excluded",
        refs_of("## 4. Heading\n\nsee ![[Page#§4]] here.\n") == [],
    )
    check(
        "image alt is excluded",
        refs_of("## 4. Heading\n\nsee ![§4](img.png) here.\n") == [],
    )
    check(
        "heading text itself is excluded",
        refs_of("## 4. Heading mentions §4\n\nno body reference.\n") == [],
    )
    check(
        "block quote content at any depth is recognized",
        refs_of("## 4. Heading\n\n> > > see §4 here\n") == [("§4", "4")],
    )

    # Resolution and diagnostics controls.
    def analyze_full(markdown: str) -> tuple[list[dict], list[dict]]:
        return analyze(markdown)

    refs, diags = analyze_full("## 4.1 Heading\n\nsee §4.1 here.\n")
    check("resolved reference has target_heading_line 1", refs[0]["target_heading_line"] == 1)
    check("resolved reference has no diagnostic", diags == [])

    refs, diags = analyze_full("## 4.1 Heading\n\nsee §99 here.\n")
    check("unresolved reference on numbered page has target None", refs[0]["target_heading_line"] is None)
    check(
        "unresolved reference on numbered page yields a diagnostic",
        diags == [{"number": "99", "line": 3}],
    )

    refs, diags = analyze_full("## Introduction\n\nsee §4.1 here.\n")
    check(
        "unresolved reference on unnumbered page has no diagnostic",
        refs[0]["target_heading_line"] is None and diags == [],
    )

    refs, diags = analyze_full(
        "## 5. First\n\ntext\n\n## 5. Second\n\nsee §5 here.\n"
    )
    check("duplicate heading number: first occurrence wins", refs[0]["target_heading_line"] == 1)

    # Range-end grammar edge cases.
    check(
        "range end rejects a double-sign continuation, producing two independent references",
        refs_of("## 3. Three\n## 5. Five\n\nsee §3–§§5 here.\n") == [("§3", "3"), ("§§5", "5")],
    )
    check(
        "range end letter lookahead rejects a following letter",
        refs_of("# Appendix A. Foo\n\nsee §A–Bz here.\n") == [("§A", "A")],
    )

    # Amendment (2026-09-24, commit a3ca8de) controls.
    check(
        "a heading inside a block quote has a section number",
        refs_of("> ## 1. Nested\n>\n> body\n\nsee §1 here.\n") == [("§1", "1")],
    )
    check(
        "a heading inside a list item has a section number",
        refs_of("- ## 1. Nested\n\nsee §1 here.\n") == [("§1", "1")],
    )
    check(
        "text between two inline HTML tags is recognized",
        refs_of("## 7. Heading\n\nsee <b>§7</b> here.\n") == [("§7", "7")],
    )
    check(
        "a comma after a range is not continued: only the range's own two refs",
        refs_of("## 3. Three\n## 7. Seven\n## 9. Nine\n\nsee §§3–7, 9 here.\n")
        == [("§§3", "3"), ("7", "7")],
    )
    check(
        "a tab after § is not the allowed space, so there is no reference",
        refs_of("## 4.1 Heading\n\nsee §\t4.1 here.\n") == [],
    )
    check(
        "a line break after a bare § ends it: no reference, on either line",
        refs_of("## 4.1 Heading\n\nsee § here\n4.1 continues here.\n") == [],
    )
    check(
        "a range never crosses a line break: only the first reference survives",
        refs_of("## 3. Three\n\nsee §3–\n5 is on the next line.\n") == [("§3", "3")],
    )

    # Corpus-wide sanity.
    cases = build_cases()
    check(f"case_count={len(CASES) + 1}", len(cases) == len(CASES) + 1)
    ids = [case["id"] for case in cases]
    check("case ids are unique", len(ids) == len(set(ids)))

    spec_case = next(case for case in cases if case["id"] == "real-spec-rha-spec-v0-10")
    check(
        "the real spec records no UnresolvedSectionRef diagnostic",
        spec_case["diagnostics"] == [],
    )
    check(
        f"the real spec yields {len(spec_case['refs'])} recognized references (informational)",
        len(spec_case["refs"]) >= 0,
    )

    return lines


# ---------------------------------------------------------------------------
# Entry point: regenerate CASES.json and selftestreport.txt.
# ---------------------------------------------------------------------------


def main() -> int:
    report_lines = run_selftests()
    cases = build_cases()

    cases_path = PACKAGE_ROOT / "CASES.json"
    cases_path.write_text(json.dumps(cases, indent=2, sort_keys=False) + "\n", encoding="utf-8")

    spec_case = next(case for case in cases if case["id"] == "real-spec-rha-spec-v0-10")
    report_lines.append(f"PASS case_count={len(cases)}")
    report_lines.append(f"PASS synthetic_case_count={len(cases) - 1}")
    report_lines.append(f"PASS spec_case_reference_count={len(spec_case['refs'])}")
    report_lines.append(f"PASS spec_case_diagnostic_count={len(spec_case['diagnostics'])}")
    report_lines.append("PASS all selftests executed without assertion failure")

    report_path = PACKAGE_ROOT / "selftestreport.txt"
    report_path.write_text("\n".join(report_lines) + "\n", encoding="utf-8")

    for line in report_lines:
        print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
