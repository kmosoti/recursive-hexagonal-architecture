#!/usr/bin/env python3
"""Independent reference implementation of the CHG-013 front-matter contract
(`docs/architecture/front-matter-contract.md`), sections 1-3, written before
any Rust implementation of the feature exists.

This module:
  * implements front-matter recognition (contract section 1);
  * implements the restricted front-matter grammar (contract section 2):
    ignored lines, key-value lines, list items, the `title` and `tags`
    recognized keys, and `Diagnostic::InvalidFrontMatter { line }`;
  * implements just enough of "the rest is markdown" (contract section 3) to
    say where body parsing starts and, for pages whose title is not given by
    front matter, what the derived title is: the first level-1 ATX heading's
    text (as `crates/document/src/parse.rs` derives it: `b.headings.iter()
    .find(|h| h.level == 1)`, else the page id's basename, `"fixture"` for
    every case in this package, since the grader always parses with a
    `Source` whose relative path is `fixture.md`);
  * defines the deterministic, hand-authored case corpus (`CASES`);
  * regenerates `CASES.json` and `selftestreport.txt` when run directly.

It deliberately does not import, read, or link against `document`,
`pulldown-cmark`, or any other crate or markdown engine in the workspace: it
knows only the contract document above. Because sections 1-2 are a
character-level, line-oriented grammar with no relation to markdown block
structure, this package needs no real markdown parser for them. For the
handful of cases that exercise section 3's "line numbers are still source
line numbers" and "title falls back to a body heading" rules, this package
recognizes only plain ATX headings (`^#{1,6} text$`, no trailing `#`
sequence, no inline markup) -- deliberately avoiding any markdown-parsing
edge case that is not itself part of the front-matter feature.
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Optional

PACKAGE_ROOT = Path(__file__).resolve().parent

# The grader always parses cases with `Source::new(RelPath::new("fixture.md"),
# markdown)`, giving `PageId` "fixture" and basename "fixture" (see
# `crates/library/src/id.rs` `PageId::from_path`/`basename`, read for this
# API fact only). Every "title derived as before, falling back to the page
# id" case in this package expects exactly this string.
FIXTURE_BASENAME = "fixture"

# ---------------------------------------------------------------------------
# Section 1: recognition.
# ---------------------------------------------------------------------------


def _delimiter_line(line: str) -> Optional[str]:
    """Returns the delimiter ("---" or "...") if `line`, with only trailing
    spaces/tabs stripped, is exactly that delimiter; else None."""
    stripped = line.rstrip(" \t")
    if stripped in ("---", "..."):
        return stripped
    return None


def find_front_matter_block(lines: list[str]) -> Optional[tuple[list[str], int]]:
    """Contract section 1. `lines` are the source's raw lines (no line
    terminators). Returns `(front_matter_lines, end_line)` when the page has
    front matter (`end_line` is the closing delimiter's 1-based line), else
    None. `front_matter_lines` are the raw lines strictly between the
    delimiters (may be empty)."""
    if not lines:
        return None
    first = lines[0]
    # A byte-order mark at the very start of the file disqualifies
    # recognition (contract section 1, paragraph on BOM); a blank first line
    # already fails the next check, since "" is never exactly "---".
    if first.startswith("﻿"):
        return None
    if _delimiter_line(first) != "---":
        return None
    for i in range(1, len(lines)):
        if _delimiter_line(lines[i]) is not None:
            return lines[1:i], i + 1
    return None


# ---------------------------------------------------------------------------
# Section 2: grammar.
# ---------------------------------------------------------------------------

KEY_VALUE_RE = re.compile(r"^([A-Za-z_][A-Za-z0-9_-]*):(.*)$")
# "- item", indented by at least one literal space (not a tab: the contract
# says "space"), the dash immediately followed by exactly one space and then
# the (possibly empty) item content.
LIST_ITEM_RE = re.compile(r"^ +- (.*)$")
BLANK_RE = re.compile(r"^[ \t]*$")
COMMENT_RE = re.compile(r"^[ \t]*#")


def _strip_matching_quotes(value: str) -> str:
    """Contract section 2: "a value wrapped in matching double or single
    quotes has the quotes removed. No escape processing is done." Used for
    `title` and for each flow-list tag element."""
    if len(value) >= 2 and value[0] == value[-1] and value[0] in ("'", '"'):
        return value[1:-1]
    return value


def _split_flow_list(inner: str) -> list[str]:
    """A flow list's inner text (between `[` and `]`), split on commas, each
    element trimmed and quote-stripped (contract section 2). This package
    does not handle a comma inside a quoted element specially (see
    README.md, open question 1)."""
    if inner.strip(" \t") == "":
        return []
    return [_strip_matching_quotes(part.strip(" \t")) for part in inner.split(",")]


def _finalize_tags(raw_items: list[str]) -> list[str]:
    """Contract section 2, "After collecting": trim, drop empty, drop an
    ASCII-case-insensitive duplicate of an earlier tag (keep the first
    spelling), preserve order of first appearance."""
    seen_casefold: set[str] = set()
    out: list[str] = []
    for raw in raw_items:
        tag = raw.strip(" \t")
        if tag == "":
            continue
        key = tag.lower()
        if key in seen_casefold:
            continue
        seen_casefold.add(key)
        out.append(tag)
    return out


class FrontMatterResult:
    def __init__(self) -> None:
        self.title: Optional[str] = None
        self.tags: list[str] = []
        self.invalid_lines: list[int] = []


def parse_front_matter_lines(fm_lines: list[str], first_line_no: int) -> FrontMatterResult:
    """Contract section 2. `fm_lines[i]` is at absolute source line
    `first_line_no + i`. Processes lines in order, applying:
      - blank/comment lines: ignored;
      - key-value lines: `title`/`tags` handled, first occurrence of each
        wins, other keys and their values/list items ignored;
      - list items: valid only when they follow (with only blank, comment,
        or invalid lines possibly between) a key-value line whose own value
        was empty -- "the most recent key-value line", recomputed on every
        key-value line regardless of key name;
      - anything else: `InvalidFrontMatter { line }`, and does not change
        what "the most recent key-value line" is.
    """
    result = FrontMatterResult()

    title_captured = False
    tags_captured = False  # the winning ("first") `tags:` occurrence has been seen
    tags_collector_active = False  # that winning occurrence had an empty value
    raw_tag_items: list[str] = []

    # `pending_key`: the key of the most recent key-value line, if and only
    # if that line's value was empty; else None.
    pending_key: Optional[str] = None

    for offset, raw_line in enumerate(fm_lines):
        line_no = first_line_no + offset

        if BLANK_RE.match(raw_line):
            continue
        if COMMENT_RE.match(raw_line):
            continue

        kv_match = KEY_VALUE_RE.match(raw_line)
        if kv_match:
            key = kv_match.group(1)
            value = kv_match.group(2).strip(" \t")
            pending_key = key if value == "" else None

            if key == "title":
                if not title_captured:
                    title_captured = True
                    stripped = _strip_matching_quotes(value)
                    result.title = stripped if stripped != "" else None
                # A later `title:` occurrence is ignored outright.
                continue

            if key == "tags":
                if not tags_captured:
                    tags_captured = True
                    if value == "":
                        tags_collector_active = True
                    else:
                        tags_collector_active = False
                        if value.startswith("[") and value.endswith("]"):
                            raw_tag_items.extend(_split_flow_list(value[1:-1]))
                        else:
                            raw_tag_items.append(value)
                else:
                    # A later `tags:` occurrence is ignored outright: its
                    # value is dropped, and even if empty, its own list
                    # items (though structurally valid) are not collected,
                    # because this occurrence is not the winning one.
                    tags_collector_active = False
                continue

            # Any other key: valid line, value and any following list items
            # ignored (contract section 2: "no diagnostic for an unknown
            # key"). `pending_key` was already updated above so a list item
            # under this key's empty value is still structurally valid.
            continue

        item_match = LIST_ITEM_RE.match(raw_line)
        if item_match:
            if pending_key is None:
                result.invalid_lines.append(line_no)
                continue
            # Valid list item line: no diagnostic, regardless of whether it
            # is collected.
            if pending_key == "tags" and tags_collector_active:
                raw_tag_items.append(item_match.group(1))
            continue

        # Neither a key-value line nor a valid list item: a nested map, an
        # unindented `- item`, a tab-indented `- item` (not "indented by at
        # least one space"), or any other unrecognized shape.
        result.invalid_lines.append(line_no)

    result.tags = _finalize_tags(raw_tag_items)
    return result


# ---------------------------------------------------------------------------
# Section 3 (just enough): body start, and title fallback via the first
# level-1 ATX heading.
# ---------------------------------------------------------------------------

ATX_HEADING_RE = re.compile(r"^(#{1,6}) (.+)$")


def scan_atx_headings(lines: list[str], first_line_no: int) -> list[dict]:
    """Plain ATX headings only (see module docstring): `level`, `text`
    (trailing spaces/tabs stripped, no other trimming), `line`. `lines[i]` is
    at absolute source line `first_line_no + i`."""
    headings = []
    for offset, raw_line in enumerate(lines):
        m = ATX_HEADING_RE.match(raw_line)
        if m:
            headings.append(
                {
                    "level": len(m.group(1)),
                    "text": m.group(2).rstrip(" \t"),
                    "line": first_line_no + offset,
                }
            )
    return headings


def analyze_page(markdown: str) -> dict:
    """Returns `{"front_matter": {...} | None, "title": str,
    "invalid_front_matter_lines": [int, ...], "headings": [{"level",
    "text", "line"}, ...]}`."""
    # `markdown` always ends with "\n" (contract cases are constructed line
    # by line); split on "\n" and drop the trailing empty element from the
    # final newline, matching how a source file's lines are counted.
    raw_lines = markdown.split("\n")
    if raw_lines and raw_lines[-1] == "":
        raw_lines = raw_lines[:-1]

    block = find_front_matter_block(raw_lines)
    if block is None:
        front_matter = None
        invalid_lines: list[int] = []
        body_first_line = 1
        body_lines = raw_lines
        title_from_front_matter = None
    else:
        fm_lines, end_line = block
        parsed = parse_front_matter_lines(fm_lines, 2)
        front_matter = {
            "title": parsed.title,
            "tags": parsed.tags,
            "end_line": end_line,
        }
        invalid_lines = parsed.invalid_lines
        body_first_line = end_line + 1
        body_lines = raw_lines[end_line:]
        title_from_front_matter = parsed.title

    headings = scan_atx_headings(body_lines, body_first_line)

    if title_from_front_matter is not None:
        title = title_from_front_matter
    else:
        first_h1 = next((h for h in headings if h["level"] == 1), None)
        title = first_h1["text"] if first_h1 is not None else FIXTURE_BASENAME

    return {
        "front_matter": front_matter,
        "title": title,
        "invalid_front_matter_lines": invalid_lines,
        "headings": headings,
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


# --- Section 1: recognition, positive -----------------------------------

add(
    "recognized-basic-title-and-tags",
    ["---", "title: Hello World", "tags: [a, b]", "---", "", "Body text."],
)
add(
    "recognized-empty-front-matter-block",
    ["---", "---", "", "# Heading"],
)
add(
    "recognized-closer-dot-dot-dot",
    ["---", "title: Via Dots", "...", "", "Body text."],
)
add(
    "recognized-opening-trailing-spaces-and-tabs",
    ["---  ", "title: Trailing", "---\t", "", "Body text."],
)
add(
    "recognized-closer-dot-dot-dot-trailing-whitespace",
    ["---", "title: Dots Trailing", "...   ", "", "Body text."],
)
add(
    "recognized-only-comments-and-blanks",
    ["---", "# just a comment", "", "   ", "---", "", "Body text."],
)
add(
    "recognized-front-matter-lines-produce-no-body-nodes",
    ["---", "title: Hidden", "tags: [x]", "---", "", "# Real Heading", "", "Body paragraph."],
)

# --- Section 1: recognition, negative (parses exactly as before) --------

add(
    "not-recognized-leading-blank-line",
    # A blank line separates "title: x" from the second "---" so that line
    # is not (mis)read as a setext-heading underline for "title: x" by a
    # real CommonMark parser; this case is only about the leading blank
    # line disqualifying recognition, not about setext headings, which this
    # package's own heading scanner does not model (see README.md).
    ["", "---", "title: x", "", "---", "", "# Heading"],
)
add(
    "not-recognized-byte-order-mark",
    # Same setext-avoidance reasoning as `not-recognized-leading-blank-line`.
    ["﻿---", "title: x", "", "---", "", "# Heading"],
)
add(
    "not-recognized-missing-closer",
    ["---", "title: x", "", "# Real Heading", "", "Body paragraph."],
)
add(
    "not-recognized-dashes-mid-page",
    # Same setext-avoidance reasoning as `not-recognized-leading-blank-line`.
    ["# Real Heading", "", "---", "title: x", "", "---", "", "Body paragraph."],
)
add(
    "not-recognized-plus-plus-plus",
    ["+++", "title: x", "+++", "", "# Heading"],
)

# --- Section 2: title -----------------------------------------------------

add(
    "title-double-quotes",
    ["---", 'title: "Quoted Title"', "---", "", "Body."],
)
add(
    "title-single-quotes",
    ["---", "title: 'Quoted Title'", "---", "", "Body."],
)
add(
    "title-empty-after-quote-removal-treated-absent",
    ["---", 'title: ""', "---", "", "# Fallback Heading"],
)
add(
    "title-empty-value-unquoted-treated-absent",
    ["---", "title:", "---", "", "# Fallback Heading"],
)
add(
    "title-duplicate-first-wins",
    ["---", "title: First", "title: Second", "---", "", "Body."],
)
add(
    "title-duplicate-first-empty-still-wins-over-nonempty-second",
    ["---", "title:", "title: Second", "---", "", "# Fallback Heading"],
)
add(
    "title-value-containing-colon",
    ["---", "title: A: B", "---", "", "Body."],
)
add(
    "title-mismatched-quotes-kept-literal",
    ["---", 'title: "Unmatched', "---", "", "Body."],
)
add(
    "title-key-is-case-sensitive-capital-t-ignored",
    ["---", "Title: Not Recognized", "---", "", "# Fallback Heading"],
)
add(
    "title-surrounded-by-comments-and-blanks",
    ["---", "# a comment before", "", "title: Between Blanks", "", "# a comment after", "---", "", "Body."],
)

# --- Section 2: tags, flow list --------------------------------------------

add(
    "tags-flow-basic",
    ["---", "tags: [alpha, beta, gamma]", "---", "", "Body."],
)
add(
    "tags-flow-quoted-element-with-space",
    ["---", 'tags: [alpha, "beta gamma"]', "---", "", "Body."],
)
add(
    "tags-flow-empty-elements-dropped",
    ["---", "tags: [alpha, , beta,  ]", "---", "", "Body."],
)
add(
    "tags-flow-ascii-case-duplicate-first-spelling-kept",
    ["---", "tags: [Alpha, alpha, ALPHA, beta]", "---", "", "Body."],
)
add(
    "tags-flow-order-is-first-appearance",
    ["---", "tags: [zeta, alpha, mu]", "---", "", "Body."],
)
add(
    "tags-flow-empty-brackets-no-tags",
    ["---", "tags: []", "---", "", "Body."],
)

# --- Section 2: tags, block list --------------------------------------------

add(
    "tags-block-basic",
    ["---", "tags:", " - alpha", " - beta", "---", "", "Body."],
)
add(
    "tags-block-empty-value-trailing-spaces",
    ["---", "tags:   ", " - alpha", "---", "", "Body."],
)
add(
    "tags-block-ascii-case-duplicate-first-spelling-kept",
    ["---", "tags:", " - Alpha", " - alpha", "---", "", "Body."],
)
add(
    "tags-block-no-items-empty-tags",
    ["---", "tags:", "other: x", "---", "", "Body."],
)
add(
    "tags-block-interrupted-by-another-key-second-item-invalid",
    ["---", "tags:", " - alpha", "other: x", " - beta", "---", "", "Body."],
)

# --- Section 2: tags, scalar -------------------------------------------------

add(
    "tags-scalar-single-tag",
    ["---", "tags: solo", "---", "", "Body."],
)
add(
    "tags-scalar-with-comma-no-brackets-is-one-literal-tag",
    ["---", "tags: alpha, beta", "---", "", "Body."],
)

# --- Section 2: tags, key-level behavior -------------------------------------

add(
    "tags-duplicate-key-first-wins-flow-then-block-ignored",
    ["---", "tags: [alpha]", "tags:", " - beta", "---", "", "Body."],
)
add(
    "tags-list-item-under-non-tags-key-ignored-value-no-diagnostic",
    ["---", "aliases:", " - old-name", "tags: [real]", "---", "", "Body."],
)

# --- Section 2: comments and blank lines -------------------------------------

add(
    "comment-lines-ignored",
    ["---", "# a full-line comment", "title: Kept", "---", "", "Body."],
)
add(
    "comment-line-with-leading-spaces",
    ["---", "   # indented comment", "title: Kept", "---", "", "Body."],
)
add(
    "blank-lines-with-only-tabs-ignored",
    ["---", "\t\t", "title: Kept", "---", "", "Body."],
)
add(
    "comments-interspersed-between-list-items",
    ["---", "tags:", " - alpha", "# a comment between items", " - beta", "---", "", "Body."],
)

# --- Section 2: unknown keys --------------------------------------------------

add(
    "unknown-key-scalar-ignored-no-diagnostic",
    ["---", "date: 2026-09-25", "title: Kept", "---", "", "Body."],
)
add(
    "unknown-key-with-block-list-ignored-no-diagnostic",
    ["---", "aliases:", " - one", " - two", "title: Kept", "---", "", "Body."],
)

# --- Section 2: invalid lines -------------------------------------------------

add(
    "invalid-nested-map",
    ["---", "outer:", "  inner: value", "---", "", "Body."],
)
add(
    "invalid-multiple-unindented-list-items",
    # Neither item is indented at all: both are unrecognized lines, and
    # neither contributes a tag.
    ["---", "tags:", "- a", "- b", "---", "", "Body."],
)
add(
    "invalid-unindented-list-item-under-empty-key",
    # The first item is properly indented and collected; the second, with no
    # leading space, is invalid and not collected.
    ["---", "tags:", " - alpha", "- unindented", "---", "", "Body."],
)
add(
    "invalid-list-item-without-preceding-empty-valued-key",
    # Properly indented, but nothing before it is a key-value line at all.
    ["---", " - stray item", "title: Kept", "---", "", "Body."],
)
add(
    "invalid-tab-indented-list-item-not-a-space",
    ["---", "tags:", "\t- tab-indented", "---", "", "Body."],
)
add(
    "invalid-key-starting-with-digit",
    ["---", "1title: x", "title: Kept", "---", "", "Body."],
)
add(
    "invalid-two-lines-report-both-in-order",
    ["---", "  bad1: x", "title: Kept", "  bad2: y", "---", "", "Body."],
)
add(
    "invalid-line-does-not-suppress-valid-keys",
    ["---", "title: Kept", "not a valid line at all", "tags: [x]", "---", "", "Body."],
)
add(
    "invalid-line-does-not-change-most-recent-key-value-line",
    ["---", "tags:", "not valid either", " - still-attached", "---", "", "Body."],
)

# --- Section 3: body start, line numbers, title fallback ---------------------

add(
    "heading-line-number-immediately-after-closer",
    ["---", "title: Has Title", "---", "# Heading Right After"],
)
add(
    "heading-line-number-after-blank-lines",
    ["---", "title: Has Title", "---", "", "", "# Heading On Line Six"],
)
add(
    "multiple-headings-line-numbers-after-front-matter",
    ["---", "tags: [x]", "---", "", "# First", "", "## Second", "", "# Third"],
)
add(
    "title-override-wins-over-body-heading",
    ["---", "title: From Front Matter", "---", "", "# Body Heading Ignored For Title"],
)
add(
    "title-falls-back-to-body-heading-when-front-matter-has-no-title",
    ["---", "tags: [x]", "---", "", "# Derived From Heading"],
)
add(
    "title-falls-back-to-basename-when-no-front-matter-title-and-no-heading",
    ["---", "tags: [x]", "---", "", "Just a paragraph, no heading."],
)
add(
    "title-not-affected-by-level-2-heading-only",
    ["---", "tags: [x]", "---", "", "## Only A Level Two Heading"],
)
add(
    "no-front-matter-title-still-derived-from-first-heading-as-before",
    ["# Ordinary First Heading", "", "Body paragraph."],
)
add(
    "no-front-matter-title-falls-back-to-basename-as-before",
    ["Just a paragraph, no heading, no front matter."],
)

# --- Combined realistic case ---------------------------------------------

add(
    "realistic-full-front-matter-title-tags-comments-blanks-invalid",
    [
        "---",
        "# a leading comment",
        'title: "A Real Page"',
        "",
        "tags:",
        " - Rust",
        " - rust",
        "aliases:",
        " - old-slug",
        "---",
        "",
        "# A Real Page",
        "",
        "Body paragraph mentioning nothing special.",
    ],
)


# ---------------------------------------------------------------------------
# Self-test
# ---------------------------------------------------------------------------


def run_selftests() -> list[str]:
    lines: list[str] = []

    def check(name: str, condition: bool) -> None:
        lines.append(f"{'PASS' if condition else 'FAIL'} {name}")
        if not condition:
            raise AssertionError(name)

    def fm(markdown: str) -> Optional[dict]:
        return analyze_page(markdown)["front_matter"]

    def title(markdown: str) -> str:
        return analyze_page(markdown)["title"]

    def invalid(markdown: str) -> list[int]:
        return analyze_page(markdown)["invalid_front_matter_lines"]

    # --- Recognition ---

    check(
        "a page opening and closing with --- has front matter",
        fm("---\n---\n") == {"title": None, "tags": [], "end_line": 2},
    )
    check(
        "a leading blank line disqualifies recognition",
        fm("\n---\n---\n") is None,
    )
    check(
        "a byte-order mark disqualifies recognition",
        fm("﻿---\n---\n") is None,
    )
    check(
        "a first line --- with no closing line is not front matter",
        fm("---\ntitle: x\n") is None,
    )
    check(
        "a --- block that does not start at line 1 is not front matter",
        fm("# H\n\n---\ntitle: x\n---\n") is None,
    )
    check("+++ is never recognized", fm("+++\ntitle: x\n+++\n") is None)
    check(
        "... closes a front-matter block",
        fm("---\ntitle: x\n...\n") == {"title": "x", "tags": [], "end_line": 3},
    )
    check(
        "trailing spaces on the opening line are allowed",
        fm("---  \n---\n") == {"title": None, "tags": [], "end_line": 2},
    )
    check(
        "trailing tabs on the closing line are allowed",
        fm("---\n---\t\n") == {"title": None, "tags": [], "end_line": 2},
    )

    # --- Title ---

    check("a double-quoted title has its quotes removed", fm('---\ntitle: "T"\n---\n')["title"] == "T")
    check("a single-quoted title has its quotes removed", fm("---\ntitle: 'T'\n---\n")["title"] == "T")
    check(
        "an empty title after quote removal is absent",
        fm('---\ntitle: ""\n---\n')["title"] is None,
    )
    check(
        "the first title occurrence wins even when empty",
        fm("---\ntitle:\ntitle: Second\n---\n")["title"] is None,
    )
    check(
        "the first non-empty title occurrence wins over a later one",
        fm("---\ntitle: First\ntitle: Second\n---\n")["title"] == "First",
    )
    check(
        "a mismatched quote is kept literal",
        fm('---\ntitle: "Unmatched\n---\n')["title"] == '"Unmatched',
    )
    check(
        "the title key is matched case-sensitively; Title is unrecognized",
        fm("---\nTitle: x\n---\n")["title"] is None,
    )

    # --- Tags ---

    check(
        "a flow list is split on commas, trimmed, and quote-stripped",
        fm('---\ntags: [a, b, "c d"]\n---\n')["tags"] == ["a", "b", "c d"],
    )
    check(
        "empty flow-list elements are dropped",
        fm("---\ntags: [a, , b,  ]\n---\n")["tags"] == ["a", "b"],
    )
    check(
        "an ASCII-case duplicate tag is dropped, first spelling kept",
        fm("---\ntags: [Alpha, alpha, ALPHA]\n---\n")["tags"] == ["Alpha"],
    )
    check(
        "a block list collects indented list items under an empty-valued tags key",
        fm("---\ntags:\n - a\n - b\n---\n")["tags"] == ["a", "b"],
    )
    check(
        "a scalar tags value is a single tag",
        fm("---\ntags: solo\n---\n")["tags"] == ["solo"],
    )
    check(
        "a scalar value with a comma but no brackets is one literal tag, not split",
        fm("---\ntags: a, b\n---\n")["tags"] == ["a, b"],
    )
    check(
        "the first tags occurrence wins; a later flow/block occurrence is ignored",
        fm("---\ntags: [a]\ntags:\n - b\n---\n")["tags"] == ["a"],
    )
    check(
        "a list item under a non-tags empty-valued key is valid but not collected",
        fm("---\naliases:\n - old\ntags: [real]\n---\n")["tags"] == ["real"],
    )
    check(
        "a list item interrupted by another key-value line stops attaching to the earlier key",
        invalid("---\ntags:\n - a\nother: x\n - b\n---\n") == [5],
    )

    # --- Invalid lines ---

    check("a nested map is invalid", invalid("---\nouter:\n  inner: x\n---\n") == [3])
    check(
        "unindented list items are invalid: the required leading space is missing",
        invalid("---\ntags:\n- a\n- b\n---\n") == [3, 4],
    )
    check(
        "a properly indented list item without any preceding empty-valued key is invalid",
        invalid("---\n - stray\ntitle: x\n---\n") == [2],
    )
    check(
        "a tab-indented list item does not count as indented by a space",
        invalid("---\ntags:\n\t- x\n---\n") == [3],
    )
    check(
        "a key starting with a digit is not a valid key-value line",
        invalid("---\n1title: x\n---\n") == [2],
    )
    check(
        "two invalid lines are both reported, in order",
        invalid("---\n  bad1: x\ntitle: x\n  bad2: y\n---\n") == [2, 4],
    )
    check(
        "front matter with invalid lines still applies its valid keys",
        fm("---\ntitle: Kept\nnot valid\ntags: [x]\n---\n") == {
            "title": "Kept",
            "tags": ["x"],
            "end_line": 5,
        },
    )
    check(
        "an invalid line does not change the most recent key-value line",
        fm("---\ntags:\nnot valid\n - still-attached\n---\n")["tags"] == ["still-attached"],
    )

    # --- Section 3 ---

    check(
        "front matter lines produce no body content; body starts after the closer",
        analyze_page("---\ntitle: x\n---\n# Heading\n")["headings"] == [
            {"level": 1, "text": "Heading", "line": 4}
        ],
    )
    check(
        "a heading's reported line is its absolute source line, after blank lines too",
        analyze_page("---\ntitle: x\n---\n\n\n# Heading\n")["headings"] == [
            {"level": 1, "text": "Heading", "line": 6}
        ],
    )
    check(
        "a front-matter title overrides a body heading for Document::title",
        title("---\ntitle: From FM\n---\n\n# Body Heading\n") == "From FM",
    )
    check(
        "without a front-matter title, the title falls back to the first level-1 heading",
        title("---\ntags: [x]\n---\n\n# Derived\n") == "Derived",
    )
    check(
        "a level-2-only heading does not supply the title",
        title("---\ntags: [x]\n---\n\n## Only H2\n") == FIXTURE_BASENAME,
    )
    check(
        "with no front-matter title and no heading, the title falls back to the basename",
        title("---\ntags: [x]\n---\n\nno heading here\n") == FIXTURE_BASENAME,
    )
    check(
        "without front matter at all, title derivation is unaffected (first heading)",
        title("# Ordinary\n\nBody.\n") == "Ordinary",
    )
    check(
        "without front matter and without a heading, title is the basename, unaffected",
        title("Just a paragraph.\n") == FIXTURE_BASENAME,
    )

    # Corpus-wide sanity.
    cases = build_cases()
    check(f"case_count={len(cases)}", len(cases) == len(CASES))
    ids = [case["id"] for case in cases]
    check("case ids are unique", len(ids) == len(set(ids)))

    return lines


def build_cases() -> list[dict]:
    result = []
    for case_id, case_lines in CASES:
        markdown = case_markdown(case_lines)
        analysis = analyze_page(markdown)
        result.append({"id": case_id, "markdown": markdown, **analysis})
    return result


def main() -> int:
    report_lines = run_selftests()
    cases = build_cases()

    cases_path = PACKAGE_ROOT / "CASES.json"
    cases_path.write_text(json.dumps(cases, indent=2, sort_keys=False) + "\n", encoding="utf-8")

    report_lines.append(f"PASS case_count={len(cases)}")
    front_matter_count = sum(1 for c in cases if c["front_matter"] is not None)
    report_lines.append(f"PASS front_matter_recognized_count={front_matter_count}")
    report_lines.append(f"PASS front_matter_not_recognized_count={len(cases) - front_matter_count}")
    invalid_total = sum(len(c["invalid_front_matter_lines"]) for c in cases)
    report_lines.append(f"PASS total_invalid_front_matter_lines={invalid_total}")
    report_lines.append("PASS all selftests executed without assertion failure")

    report_path = PACKAGE_ROOT / "selftestreport.txt"
    report_path.write_text("\n".join(report_lines) + "\n", encoding="utf-8")

    for line in report_lines:
        print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
