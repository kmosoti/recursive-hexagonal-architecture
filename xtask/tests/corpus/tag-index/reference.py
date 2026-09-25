#!/usr/bin/env python3
"""Independent reference implementation of the CHG-013 stage B tag-index
contract (`docs/architecture/tag-index-contract.md`), sections 1-3, written
before any Rust implementation of the feature exists.

This module:
  * re-derives just enough of the CHG-013 stage A front-matter grammar
    (`docs/architecture/front-matter-contract.md`, sections 1-2, already
    implemented) to read a page's `title`, `tags` and new `index` key --
    the tag-index contract is defined in terms of these fields, so an
    independent site-level oracle for stage B must compute them itself
    rather than trust the tool under test;
  * re-derives `document::slugify` (`crates/document/src/slug.rs`, read
    for this already-public, documented function only) to compute both a
    page's own heading slugs and each generated tag anchor;
  * derives each page's title exactly as `document::parse` does it
    (front-matter title, else the first level-1 ATX heading, else the page
    id's basename -- `crates/document/src/parse.rs`, read for this API
    fact only, restricted here to plain ATX headings, as in the stage A
    oracle package `xtask/tests/corpus/front-matter/reference.py`);
  * implements contract sections 2-3: the site's tags (identity, display
    spelling, page order) and, for each tag-index page, the generated
    section (heading, list, anchor de-duplication) and TOC tail;
  * defines the deterministic, hand-authored case corpus (`CASES`);
  * regenerates `CASES.json` and `selftestreport.txt` when run directly.

It deliberately does not import, read, or link against `document`, `site`,
`pulldown-cmark`, or any renderer crate: it knows only the two contracts
above and the two named, narrow, already-public/already-stable facts (the
slug algorithm and `document::parse`'s title-derivation order). It builds
no HTML or JSON: the app-cli grader (`crates/app-cli/tests/tag_index.rs`)
runs the real CLI and checks its JSON/HTML output against this package's
`CASES.json`.

Restrictions deliberately kept out of scope, so this package needs no real
markdown parser (see README.md for open questions this leaves out of the
corpus rather than guessing):
  * page bodies use only plain ATX headings (`^#{1,6} text$`); no setext
    headings, no inline markup in heading text;
  * no page in this corpus has two headings whose base slugs collide with
    each other (i.e. a page's own heading slugs need no de-duplication
    among themselves); this keeps "the page's own heading slugs" a simple,
    unambiguous set for the anchor-collision rule in contract section 3,
    without this package having to reproduce `document::parse`'s
    heading-de-duplication algorithm (an unrelated, already-implemented
    detail of an existing feature);
  * front matter in this corpus is always well-formed under the stage A
    grammar (no `InvalidFrontMatter` lines): stage A's own grammar
    corpus already owns that surface.
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Optional

PACKAGE_ROOT = Path(__file__).resolve().parent

PACKAGE_FILES = (
    "CASES.json",
    "README.md",
    "SHA256SUMS",
    "reference.py",
    "registration.toml",
    "selftestreport.txt",
    "source-snapshots/contract.md",
    "source-snapshots/prompt.md",
)

# ---------------------------------------------------------------------------
# document::slugify port (crates/document/src/slug.rs). Read for its public,
# documented rule only: Unicode-lowercase, keep letters/digits/space/-/_,
# space -> '-', no collapsing, no NFC.
# ---------------------------------------------------------------------------


def slugify(text: str) -> str:
    out = []
    for ch in text.lower():
        if ch.isalnum() or ch in (" ", "-", "_"):
            out.append("-" if ch == " " else ch)
    return "".join(out)


# ---------------------------------------------------------------------------
# Stage A front matter (contract sections 1-2), re-derived just enough to
# read `title`, `tags`, and the new `index` key. See
# `xtask/tests/corpus/front-matter/reference.py` for the full stage A
# oracle; this is a narrower, corpus-local re-derivation from the contract
# text, kept in sync with it deliberately (this package's corpus never
# exercises the stage A grammar's error surface -- that is stage A's own
# corpus's job).
# ---------------------------------------------------------------------------

KEY_VALUE_RE = re.compile(r"^([A-Za-z_][A-Za-z0-9_-]*):(.*)$")
LIST_ITEM_RE = re.compile(r"^ +- (.*)$")
BLANK_RE = re.compile(r"^[ \t]*$")
COMMENT_RE = re.compile(r"^[ \t]*#")
ATX_HEADING_RE = re.compile(r"^(#{1,6}) (.+)$")


def _delimiter_line(line: str) -> Optional[str]:
    stripped = line.rstrip(" \t")
    if stripped in ("---", "..."):
        return stripped
    return None


def _strip_matching_quotes(value: str) -> str:
    if len(value) >= 2 and value[0] == value[-1] and value[0] in ("'", '"'):
        return value[1:-1]
    return value


def _split_flow_list(inner: str) -> list[str]:
    if inner.strip(" \t") == "":
        return []
    return [_strip_matching_quotes(part.strip(" \t")) for part in inner.split(",")]


def _finalize_tags(raw_items: list[str]) -> list[str]:
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


class FrontMatter:
    def __init__(self) -> None:
        self.title: Optional[str] = None
        self.tags: list[str] = []
        self.index: Optional[str] = None
        self.end_line = 0


def find_front_matter_block(lines: list[str]) -> Optional[tuple[list[str], int]]:
    if not lines:
        return None
    first = lines[0]
    if first.startswith("﻿"):
        return None
    if _delimiter_line(first) != "---":
        return None
    for i in range(1, len(lines)):
        if _delimiter_line(lines[i]) is not None:
            return lines[1:i], i + 1
    return None


def parse_front_matter_lines(fm_lines: list[str]) -> FrontMatter:
    """Contract section 2 (front-matter-contract.md) plus tag-index
    contract section 1's `index` key: a scalar, quotes removed as for
    `title`, first occurrence among valid lines wins, empty after quote
    removal counts as absent."""
    fm = FrontMatter()
    title_captured = False
    tags_captured = False
    tags_collector_active = False
    index_captured = False
    raw_tag_items: list[str] = []
    pending_key: Optional[str] = None

    for raw_line in fm_lines:
        if BLANK_RE.match(raw_line) or COMMENT_RE.match(raw_line):
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
                    fm.title = stripped if stripped != "" else None
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
                    tags_collector_active = False
                continue

            if key == "index":
                if not index_captured:
                    index_captured = True
                    stripped = _strip_matching_quotes(value)
                    fm.index = stripped if stripped != "" else None
                continue

            # Any other key: valid, ignored.
            continue

        item_match = LIST_ITEM_RE.match(raw_line)
        if item_match:
            if pending_key == "tags" and tags_collector_active:
                raw_tag_items.append(item_match.group(1))
            continue

        # Invalid line: this corpus never exercises this path (see module
        # docstring); ignored here since diagnostics are stage A's surface.

    fm.tags = _finalize_tags(raw_tag_items)
    return fm


def scan_atx_headings(lines: list[str]) -> list[dict]:
    headings = []
    for raw_line in lines:
        m = ATX_HEADING_RE.match(raw_line)
        if m:
            headings.append({"level": len(m.group(1)), "text": m.group(2).rstrip(" \t")})
    return headings


class PageAnalysis:
    def __init__(self, page_id: str, front_matter: Optional[FrontMatter], title: str, heading_slugs: list[str]):
        self.page_id = page_id
        self.front_matter = front_matter
        self.title = title
        self.heading_slugs = heading_slugs

    @property
    def is_tag_index(self) -> bool:
        return self.front_matter is not None and self.front_matter.index == "tags"

    @property
    def tags(self) -> list[str]:
        return self.front_matter.tags if self.front_matter is not None else []


def basename(page_id: str) -> str:
    return page_id.rsplit("/", 1)[-1]


def analyze_page(page_id: str, markdown: str) -> PageAnalysis:
    raw_lines = markdown.split("\n")
    if raw_lines and raw_lines[-1] == "":
        raw_lines = raw_lines[:-1]

    block = find_front_matter_block(raw_lines)
    if block is None:
        front_matter = None
        body_lines = raw_lines
    else:
        fm_lines, end_line = block
        front_matter = parse_front_matter_lines(fm_lines)
        front_matter.end_line = end_line
        body_lines = raw_lines[end_line:]

    headings = scan_atx_headings(body_lines)
    heading_slugs = [slugify(h["text"]) for h in headings]
    # This corpus never gives a page two headings with the same base slug
    # (module docstring); guarded here so a corpus bug fails loudly instead
    # of silently under-specifying "the page's own heading slugs".
    assert len(heading_slugs) == len(set(heading_slugs)), (
        f"{page_id}: corpus restriction violated -- headings must have distinct slugs"
    )

    if front_matter is not None and front_matter.title is not None:
        title = front_matter.title
    else:
        first_h1 = next((h for h in headings if h["level"] == 1), None)
        title = first_h1["text"] if first_h1 is not None else basename(page_id)

    return PageAnalysis(page_id, front_matter, title, heading_slugs)


# ---------------------------------------------------------------------------
# Tag-index contract sections 2-3.
# ---------------------------------------------------------------------------


class TagGroup:
    def __init__(self, display: str):
        self.display = display
        self.page_ids: list[str] = []


def site_tags(pages: dict[str, PageAnalysis]) -> list[TagGroup]:
    """Contract section 2. `pages` maps page id -> analysis; iteration below
    is explicitly in page-id order, independent of dict insertion order."""
    groups: dict[str, TagGroup] = {}  # keyed by ascii-lowercased identity
    for page_id in sorted(pages):
        page = pages[page_id]
        for tag in page.tags:
            key = ascii_lower(tag)
            if key not in groups:
                groups[key] = TagGroup(display=tag)
            group = groups[key]
            if page_id not in group.page_ids:
                group.page_ids.append(page_id)
    # `page_ids` was appended in page-id-ascending order because the outer
    # loop already iterates `sorted(pages)`.
    return list(groups.values())


def ascii_lower(text: str) -> str:
    """ASCII-only lowercasing: only A-Z is affected, matching "equal
    ignoring ASCII case" (front-matter-contract.md stage A rule, reused by
    tag-index-contract.md section 2)."""
    return "".join(chr(ord(c) + 32) if "A" <= c <= "Z" else c for c in text)


def sort_tags(groups: list[TagGroup]) -> list[TagGroup]:
    """Contract section 3: "sorted by their ASCII-lowercased display
    spelling. Ties are broken by display spelling, then byte order." Given
    section 2's identity rule (equal ignoring ASCII case), two distinct
    groups always have distinct ASCII-lowercased spellings, so this
    tie-break is unreachable from any site this package can construct; see
    README.md."""
    return sorted(groups, key=lambda g: (ascii_lower(g.display), g.display))


def anchor_for_tag(display: str, page_heading_slugs: set[str], assigned: set[str]) -> str:
    """Contract section 3, "Anchors": `tag-` + slugify(display), or `tag`
    when that slug is empty; collisions (against the page's own heading
    slugs, or a slug already given to an earlier tag on this page) get the
    first unused `-2`, `-3`, ... suffix, checked against the union of both
    sets."""
    base_slug = slugify(display)
    base = f"tag-{base_slug}" if base_slug != "" else "tag"
    if base not in page_heading_slugs and base not in assigned:
        return base
    n = 2
    while True:
        candidate = f"{base}-{n}"
        if candidate not in page_heading_slugs and candidate not in assigned:
            return candidate
        n += 1


class GeneratedSection:
    def __init__(self, heading_text: str, slug: str, links: list[dict]):
        self.heading_text = heading_text
        self.slug = slug
        self.links = links  # [{"page_id": ..., "link_text": ...}]


def generated_sections_for_page(
    page: PageAnalysis, tags_in_order: list[TagGroup], pages: dict[str, PageAnalysis]
) -> list[GeneratedSection]:
    assigned: set[str] = set()
    page_heading_slugs = set(page.heading_slugs)
    sections = []
    for group in tags_in_order:
        slug = anchor_for_tag(group.display, page_heading_slugs, assigned)
        assigned.add(slug)
        links = [
            {"page_id": target_id, "link_text": pages[target_id].title} for target_id in group.page_ids
        ]
        sections.append(GeneratedSection(heading_text=group.display, slug=slug, links=links))
    return sections


def analyze_site(pages_markdown: dict[str, str]) -> dict:
    pages = {page_id: analyze_page(page_id, markdown) for page_id, markdown in pages_markdown.items()}
    tags_in_order = sort_tags(site_tags(pages))
    expected_index_pages = {}
    if tags_in_order:
        for page_id in sorted(pages):
            page = pages[page_id]
            if not page.is_tag_index:
                continue
            sections = generated_sections_for_page(page, tags_in_order, pages)
            expected_index_pages[page_id] = {
                "sections": [
                    {
                        "heading_text": s.heading_text,
                        "slug": s.slug,
                        "links": s.links,
                    }
                    for s in sections
                ],
                "toc_tail": [
                    {"level": 2, "text": s.heading_text, "anchor": s.slug} for s in sections
                ],
            }
    else:
        # "When the site has no tags, nothing is appended" -- including on a
        # page whose front matter opts in.
        for page_id in sorted(pages):
            if pages[page_id].is_tag_index:
                expected_index_pages[page_id] = {"sections": [], "toc_tail": []}
    return {"pages": pages, "expected_index_pages": expected_index_pages}


# ---------------------------------------------------------------------------
# Case corpus.
# ---------------------------------------------------------------------------

CASES: list[tuple[str, dict[str, str]]] = []


def add(case_id: str, pages: dict[str, str]) -> None:
    for existing_id, _ in CASES:
        assert existing_id != case_id, f"duplicate case id {case_id}"
    CASES.append((case_id, pages))


def md(*lines: str) -> str:
    return "\n".join(lines) + "\n"


# 1. No tags anywhere, no index page.
add(
    "no-tags-no-index-page",
    {
        "solo": md("# Solo Page", "", "Just a page, no front matter at all."),
    },
)

# 2. Tags absent, an index page opts in: nothing appended.
add(
    "tags-absent-index-page-present-nothing-appended",
    {
        "hub": md("---", "index: tags", "---", "", "# Hub", "", "No tags exist on this site."),
        "plain": md("# Plain", "", "No front matter, no tags."),
    },
)

# 3. Tags exist, no page opts in: nothing appended anywhere.
add(
    "tags-present-no-index-page-nothing-appended",
    {
        "alpha": md("---", "tags: [Red]", "---", "", "# Alpha", "", "Body."),
        "beta": md("---", "tags: [Blue]", "---", "", "# Beta", "", "Body."),
    },
)

# 4. Two index pages both get the identical generated sections.
add(
    "two-index-pages-both-get-sections",
    {
        "idx-one": md("---", "index: tags", "---", "", "# Index One", "", "First index page."),
        "idx-two": md("---", "index: tags", "---", "", "# Index Two", "", "Second index page."),
        "alpha": md("---", "tags: [Red]", "---", "", "# Alpha", "", "Body."),
        "beta": md("---", "tags: [Blue]", "---", "", "# Beta", "", "Body."),
    },
)

# 5. An index page that is itself tagged appears in its own tag's list.
add(
    "index-page-itself-tagged",
    {
        "hub": md("---", "index: tags", "tags: [Docs]", "---", "", "# Hub", "", "Body."),
        "leaf": md("---", "tags: [Docs]", "---", "", "# Leaf", "", "Body."),
    },
)

# 6. ASCII-case identity merges tags spelled differently across pages.
add(
    "ascii-case-identity-merges-across-pages",
    {
        "a-page": md("---", "tags: [Alpha]", "---", "", "# A Page", "", "Body."),
        "b-page": md("---", "tags: [ALPHA]", "---", "", "# B Page", "", "Body."),
        "hub": md("---", "index: tags", "---", "", "# Hub", "", "Body."),
    },
)

# 7. Display spelling is from the first page in page-id order, not file/map
# order: "aaa-page" sorts before "zzz-page" even though this case lists
# "zzz-page" first.
add(
    "display-spelling-from-first-page-in-page-id-order",
    {
        "zzz-page": md("---", "tags: [TEST]", "---", "", "# Zzz Page", "", "Body."),
        "aaa-page": md("---", "tags: [test]", "---", "", "# Aaa Page", "", "Body."),
        "hub": md("---", "index: tags", "---", "", "# Hub", "", "Body."),
    },
)

# 8. Non-ASCII tag (CJK): slugify keeps the characters, lowercased.
add(
    "non-ascii-tag-cjk",
    {
        "hub": md("---", "index: tags", "---", "", "# Hub", "", "Body."),
        "leaf": md("---", 'tags: ["日本語"]', "---", "", "# Leaf", "", "Body."),
    },
)

# 9. Non-ASCII tag (accented Latin): slugify lowercases, keeps letters.
add(
    "non-ascii-tag-accented-cafe",
    {
        "hub": md("---", "index: tags", "---", "", "# Hub", "", "Body."),
        "leaf": md("---", "tags: [Café]", "---", "", "# Leaf", "", "Body."),
    },
)

# 10. A tag whose slugify is empty (only filtered punctuation) anchors at
# plain "tag".
add(
    "tag-slug-empty-from-symbols-only",
    {
        "hub": md("---", "index: tags", "---", "", "# Hub", "", "Body."),
        "leaf": md("---", "tags: ['!!!']", "---", "", "# Leaf", "", "Body."),
    },
)

# 11. Two tags that both slugify empty: the first gets "tag", the second
# "tag-2" (collision against an earlier tag's assigned slug).
add(
    "two-empty-slug-tags-dedup-tag-and-tag-2",
    {
        "hub": md("---", "index: tags", "---", "", "# Hub", "", "Body."),
        "leaf": md("---", "tags: ['!!!', '@@@']", "---", "", "# Leaf", "", "Body."),
    },
)

# 12. A tag's natural anchor collides with one of the index page's own
# heading slugs: "Rust" -> "tag-rust" collides with heading "Tag Rust" ->
# slug "tag-rust", so the tag gets "tag-rust-2".
add(
    "tag-slug-collides-with-page-own-heading-slug",
    {
        "hub": md(
            "---",
            "index: tags",
            "---",
            "",
            "# Hub",
            "",
            "## Tag Rust",
            "",
            "Some unrelated section that happens to slug to tag-rust.",
        ),
        "leaf": md("---", "tags: [Rust]", "---", "", "# Leaf", "", "Body."),
    },
)

# 13. Two different tags whose natural anchors collide with each other:
# "C" -> "tag-c"; "C++" -> "tag-c" too (slugify drops '+'). Sorted by
# ASCII-lowercased spelling, "c" < "c++", so "C" is placed first and keeps
# "tag-c"; "C++" gets "tag-c-2".
add(
    "two-tags-collide-with-each-other-suffix-2",
    {
        "hub": md("---", "index: tags", "---", "", "# Hub", "", "Body."),
        "leaf": md("---", "tags: [C, 'C++']", "---", "", "# Leaf", "", "Body."),
    },
)

# 14. Three tags whose natural anchors all collide ("C", "C++", "C?!" all
# slugify to "c"): the third skips both "tag-c" and "tag-c-2" and takes
# "tag-c-3" -- "the first unused one", not a restart.
add(
    "three-tags-collide-skip-to-first-unused-suffix-3",
    {
        "hub": md("---", "index: tags", "---", "", "# Hub", "", "Body."),
        "leaf": md("---", "tags: [C, 'C++', 'C?!']", "---", "", "# Leaf", "", "Body."),
    },
)

# 15. Pages in subdirectories: page ids compare byte-wise, "docs/intro" <
# "docs/setup/step1".
add(
    "pages-in-subdirectories",
    {
        "hub": md("---", "index: tags", "---", "", "# Hub", "", "Body."),
        "docs/intro": md("---", "tags: [Guide]", "---", "", "# Intro", "", "Body."),
        "docs/setup/step1": md("---", "tags: [Guide]", "---", "", "# Step One", "", "Body."),
    },
)

# 16. `index: "tags"`, quoted, is still recognized (quotes removed as for
# `title`).
add(
    "index-quoted-value-tags",
    {
        "hub": md("---", 'index: "tags"', "---", "", "# Hub", "", "Body."),
        "leaf": md("---", "tags: [Docs]", "---", "", "# Leaf", "", "Body."),
    },
)

# 17. `index: other` has no effect: not a tag index page.
add(
    "index-other-value-no-effect",
    {
        "hub": md("---", "index: other", "---", "", "# Hub", "", "Body."),
        "leaf": md("---", "tags: [Docs]", "---", "", "# Leaf", "", "Body."),
    },
)

# 18. `index:` with an empty value counts as absent: not a tag index page.
add(
    "index-empty-value-absent",
    {
        "hub": md("---", "index:", "---", "", "# Hub", "", "Body."),
        "leaf": md("---", "tags: [Docs]", "---", "", "# Leaf", "", "Body."),
    },
)

# 19. Two `index:` lines, "other" then "tags": the first occurrence wins,
# so this page is NOT a tag index.
add(
    "index-first-occurrence-wins-other-beats-later-tags",
    {
        "hub": md("---", "index: other", "index: tags", "---", "", "# Hub", "", "Body."),
        "leaf": md("---", "tags: [Docs]", "---", "", "# Leaf", "", "Body."),
    },
)

# 20. Two `index:` lines, "tags" then "other": the first occurrence wins,
# so this page IS a tag index.
add(
    "index-first-occurrence-wins-tags-beats-later-other",
    {
        "hub": md("---", "index: tags", "index: other", "---", "", "# Hub", "", "Body."),
        "leaf": md("---", "tags: [Docs]", "---", "", "# Leaf", "", "Body."),
    },
)

# 21. `index` is a scalar key: `[tags]`-shaped flow-list syntax is not
# special-cased for it (only `tags` gets flow-list parsing), so the raw
# value "[tags]" != "tags" and the page is not a tag index.
add(
    "index-value-flow-list-syntax-not-special-cased",
    {
        "hub": md("---", "index: [tags]", "---", "", "# Hub", "", "Body."),
        "leaf": md("---", "tags: [Docs]", "---", "", "# Leaf", "", "Body."),
    },
)

# 22. Generated link text uses `Document::title`: a front-matter title.
add(
    "link-text-uses-front-matter-title",
    {
        "hub": md("---", "index: tags", "---", "", "# Hub", "", "Body."),
        "leaf": md(
            "---",
            'title: "My Custom Title"',
            "tags: [Docs]",
            "---",
            "",
            "# Leaf Heading Ignored For Title",
            "",
            "Body.",
        ),
    },
)

# 23. Generated link text falls back to the first level-1 heading when
# there is no front-matter title.
add(
    "link-text-uses-derived-first-heading-title",
    {
        "hub": md("---", "index: tags", "---", "", "# Hub", "", "Body."),
        "leaf2": md("---", "tags: [Docs2]", "---", "", "# Real Heading", "", "Body."),
    },
)

# 24. Generated link text falls back to the page id's basename when there
# is neither a front-matter title nor any heading.
add(
    "link-text-falls-back-to-basename-title",
    {
        "hub": md("---", "index: tags", "---", "", "# Hub", "", "Body."),
        "leaf3": md("---", "tags: [Docs3]", "---", "", "Just a paragraph, no heading."),
    },
)

# 25. The index page already has a body wikilink before any generated
# content: the generated links must be appended after it (new `links`
# entries only), not interleaved or renumbered.
add(
    "existing-body-links-precede-generated-link-indices",
    {
        "hub": md(
            "---",
            "index: tags",
            "---",
            "",
            "# Hub",
            "",
            "See also [[leaf3]].",
        ),
        "leaf3": md("---", "tags: [Zeta]", "---", "", "# Leaf Three", "", "Body."),
    },
)

# 26. Multiple tags per page and multiple pages per tag (a small cross
# product), exercising ordinary tag ordering (blue < green < red) and
# each tag's own page-id-ordered page list.
add(
    "multiple-tags-multiple-pages-cross-product",
    {
        "hub": md("---", "index: tags", "---", "", "# Hub", "", "Body."),
        "page-a": md("---", "tags: [Red, Blue]", "---", "", "# Page A", "", "Body."),
        "page-b": md("---", "tags: [Blue, Green]", "---", "", "# Page B", "", "Body."),
        "page-c": md("---", "tags: [Red]", "---", "", "# Page C", "", "Body."),
    },
)

# 27. Tags collected regardless of whether the source page used the flow-
# list or block-list form (stage A grammar forms), merged into one tag.
add(
    "tags-collected-regardless-of-flow-or-block-list-form",
    {
        "hub": md("---", "index: tags", "---", "", "# Hub", "", "Body."),
        "leaf-flow": md("---", "tags: [Foo]", "---", "", "# Leaf Flow", "", "Body."),
        "leaf-block": md("---", "tags:", " - Foo", "---", "", "# Leaf Block", "", "Body."),
    },
)


# ---------------------------------------------------------------------------
# Self-test.
# ---------------------------------------------------------------------------


def run_selftests() -> list[str]:
    lines: list[str] = []

    def check(name: str, condition: bool) -> None:
        lines.append(f"{'PASS' if condition else 'FAIL'} {name}")
        if not condition:
            raise AssertionError(name)

    check("slugify keeps the contract example", slugify("Hello, World!") == "hello-world")
    check("slugify collapses nothing", slugify("a  b") == "a--b")
    check("slugify drops filtered punctuation entirely", slugify("C++ and `code`") == "c-and-code")
    check("slugify keeps non-ASCII letters, lowercased", slugify("Café") == "café")
    check("slugify of only punctuation is empty", slugify("!!!") == "")

    check("ascii_lower only touches A-Z", ascii_lower("CafÉ") == "cafÉ")

    hub_only = {"hub": analyze_page("hub", CASES_BY_ID["two-index-pages-both-get-sections"]["idx-one"])}
    check("a plain index page has no tags", hub_only["hub"].tags == [])
    check("a plain index page is a tag index", hub_only["hub"].is_tag_index)

    site = analyze_site(CASES_BY_ID["ascii-case-identity-merges-across-pages"])
    groups = sort_tags(site_tags(site["pages"]))
    check("ASCII-case identity merges into one group", len(groups) == 1)
    check("display spelling is the first page's spelling (a-page)", groups[0].display == "Alpha")
    check(
        "pages of the tag are listed in page-id order",
        groups[0].page_ids == ["a-page", "b-page"],
    )

    order_site = analyze_site(CASES_BY_ID["display-spelling-from-first-page-in-page-id-order"])
    order_groups = sort_tags(site_tags(order_site["pages"]))
    check(
        "display spelling comes from the page-id-first page, not map order",
        order_groups[0].display == "test",
    )

    empty_site = analyze_site(CASES_BY_ID["two-empty-slug-tags-dedup-tag-and-tag-2"])
    sections = empty_site["expected_index_pages"]["hub"]["sections"]
    check("first empty-slug tag anchors at 'tag'", sections[0]["slug"] == "tag")
    check("second empty-slug tag anchors at 'tag-2'", sections[1]["slug"] == "tag-2")

    heading_collide_site = analyze_site(CASES_BY_ID["tag-slug-collides-with-page-own-heading-slug"])
    hc_sections = heading_collide_site["expected_index_pages"]["hub"]["sections"]
    check(
        "a tag colliding with the page's own heading slug gets -2",
        hc_sections[0]["slug"] == "tag-rust-2",
    )

    three_way_site = analyze_site(CASES_BY_ID["three-tags-collide-skip-to-first-unused-suffix-3"])
    tw_sections = three_way_site["expected_index_pages"]["hub"]["sections"]
    check(
        "three colliding tags occupy tag-c, tag-c-2, tag-c-3 in tag order",
        [s["slug"] for s in tw_sections] == ["tag-c", "tag-c-2", "tag-c-3"],
    )

    no_tags_index_site = analyze_site(
        CASES_BY_ID["tags-absent-index-page-present-nothing-appended"]
    )
    check(
        "a tag index page on a site with no tags gets an empty generated section",
        no_tags_index_site["expected_index_pages"]["hub"] == {"sections": [], "toc_tail": []},
    )

    no_index_site = analyze_site(CASES_BY_ID["tags-present-no-index-page-nothing-appended"])
    check(
        "a site with tags but no index page appends nothing anywhere",
        no_index_site["expected_index_pages"] == {},
    )

    two_idx_site = analyze_site(CASES_BY_ID["two-index-pages-both-get-sections"])
    check(
        "two index pages get identical generated sections",
        two_idx_site["expected_index_pages"]["idx-one"]
        == two_idx_site["expected_index_pages"]["idx-two"],
    )

    self_tagged_site = analyze_site(CASES_BY_ID["index-page-itself-tagged"])
    check(
        "an index page that carries the tag is listed under its own generated section",
        self_tagged_site["expected_index_pages"]["hub"]["sections"][0]["links"][0]["page_id"]
        == "hub",
    )

    quoted_site = analyze_site(CASES_BY_ID["index-quoted-value-tags"])
    check("a quoted index: \"tags\" value is recognized", "hub" in quoted_site["expected_index_pages"])

    other_site = analyze_site(CASES_BY_ID["index-other-value-no-effect"])
    check("index: other is not a tag index page", other_site["expected_index_pages"] == {})

    empty_idx_site = analyze_site(CASES_BY_ID["index-empty-value-absent"])
    check("index: (empty) is not a tag index page", empty_idx_site["expected_index_pages"] == {})

    first_wins_other_site = analyze_site(
        CASES_BY_ID["index-first-occurrence-wins-other-beats-later-tags"]
    )
    check(
        "first index: occurrence wins even when a later one says tags",
        first_wins_other_site["expected_index_pages"] == {},
    )

    first_wins_tags_site = analyze_site(
        CASES_BY_ID["index-first-occurrence-wins-tags-beats-later-other"]
    )
    check(
        "first index: occurrence wins when it says tags",
        "hub" in first_wins_tags_site["expected_index_pages"],
    )

    flow_site = analyze_site(CASES_BY_ID["index-value-flow-list-syntax-not-special-cased"])
    check(
        "index: [tags] is a literal scalar, not special-cased flow-list syntax",
        flow_site["expected_index_pages"] == {},
    )

    fm_title_site = analyze_site(CASES_BY_ID["link-text-uses-front-matter-title"])
    check(
        "generated link text uses the front-matter title",
        fm_title_site["expected_index_pages"]["hub"]["sections"][0]["links"][0]["link_text"]
        == "My Custom Title",
    )

    heading_title_site = analyze_site(CASES_BY_ID["link-text-uses-derived-first-heading-title"])
    check(
        "generated link text falls back to the first level-1 heading",
        heading_title_site["expected_index_pages"]["hub"]["sections"][0]["links"][0]["link_text"]
        == "Real Heading",
    )

    basename_title_site = analyze_site(CASES_BY_ID["link-text-falls-back-to-basename-title"])
    check(
        "generated link text falls back to the page id's basename",
        basename_title_site["expected_index_pages"]["hub"]["sections"][0]["links"][0]["link_text"]
        == "leaf3",
    )

    cross_site = analyze_site(CASES_BY_ID["multiple-tags-multiple-pages-cross-product"])
    cross_sections = cross_site["expected_index_pages"]["hub"]["sections"]
    check(
        "tag order is blue < green < red by ASCII-lowercased spelling",
        [s["heading_text"] for s in cross_sections] == ["Blue", "Green", "Red"],
    )
    check(
        "Blue lists page-a then page-b, in page-id order",
        [link["page_id"] for link in cross_sections[0]["links"]] == ["page-a", "page-b"],
    )
    check(
        "Red lists page-a then page-c, in page-id order",
        [link["page_id"] for link in cross_sections[2]["links"]] == ["page-a", "page-c"],
    )

    block_flow_site = analyze_site(
        CASES_BY_ID["tags-collected-regardless-of-flow-or-block-list-form"]
    )
    bf_sections = block_flow_site["expected_index_pages"]["hub"]["sections"]
    check(
        "flow-list and block-list tag forms merge into one tag with both pages",
        len(bf_sections) == 1 and [l["page_id"] for l in bf_sections[0]["links"]]
        == ["leaf-block", "leaf-flow"],
    )

    cases = build_cases()
    check(f"case_count={len(cases)}", len(cases) == len(CASES))
    ids = [case["id"] for case in cases]
    check("case ids are unique", len(ids) == len(set(ids)))
    check("at least 25 cases are registered", len(cases) >= 25)

    return lines


def build_cases() -> list[dict]:
    result = []
    for case_id, pages_markdown in CASES:
        site = analyze_site(pages_markdown)
        result.append(
            {
                "id": case_id,
                "pages": pages_markdown,
                "expected_index_pages": site["expected_index_pages"],
            }
        )
    return result


CASES_BY_ID = {case_id: pages for case_id, pages in CASES}


def json_bytes(cases: list[dict]) -> bytes:
    return (json.dumps(cases, indent=2, sort_keys=False) + "\n").encode("utf-8")


def self_test() -> tuple[bool, str]:
    try:
        report_lines = run_selftests()
    except AssertionError as exc:
        return False, f"FAIL\n- {exc}\n"

    errors: list[str] = []
    for relative in PACKAGE_FILES:
        if not (PACKAGE_ROOT / relative).is_file():
            errors.append(f"package file is missing: {relative}")

    cases = build_cases()
    case_file = PACKAGE_ROOT / "CASES.json"
    if case_file.is_file():
        actual = case_file.read_bytes()
        expected = json_bytes(cases)
        if actual != expected:
            errors.append("CASES.json differs from deterministic generator output")
    else:
        errors.append("CASES.json is missing")

    if errors:
        return False, "FAIL\n" + "\n".join(f"- {e}" for e in errors) + "\n"

    tag_count = len(
        {
            ascii_lower(tag)
            for _, pages in CASES
            for page_id, markdown in pages.items()
            for tag in analyze_page(page_id, markdown).tags
        }
    )
    index_page_count = sum(len(case["expected_index_pages"]) for case in cases)
    return True, (
        "PASS\n"
        f"- {len(cases)} site cases registered\n"
        f"- {sum(len(c['pages']) for c in cases)} pages across all cases\n"
        f"- {tag_count} distinct ASCII-case-insensitive tag identities exercised across the corpus\n"
        f"- {index_page_count} (case, tag-index-page) generated-section observations\n"
        f"- {len(report_lines)} self-test assertions, all PASS\n"
        "- no target implementation, renderer, network, or live oracle was invoked\n"
    )


def reproduce(destination: str) -> int:
    requested = Path(destination)
    if requested.is_absolute() or len(requested.parts) != 1 or requested.parts[0] in {".", ".."}:
        print("refusing reproduction: destination must be one fresh direct child", file=sys.stderr)
        return 2
    out = PACKAGE_ROOT / requested.parts[0]
    if out.exists() and (not out.is_dir() or any(out.iterdir())):
        print("refusing reproduction: child directory must be fresh and empty", file=sys.stderr)
        return 2
    out.mkdir()
    for relative in PACKAGE_FILES:
        destination_file = out / relative
        destination_file.parent.mkdir(parents=True, exist_ok=True)
        if relative == "CASES.json":
            destination_file.write_bytes(json_bytes(build_cases()))
        else:
            import shutil

            shutil.copyfile(PACKAGE_ROOT / relative, destination_file)
    mismatched = [
        relative
        for relative in PACKAGE_FILES
        if (PACKAGE_ROOT / relative).read_bytes() != (out / relative).read_bytes()
    ]
    if mismatched:
        print(f"reproduction byte comparison failed: {mismatched}", file=sys.stderr)
        return 1
    print(f"reproduced {len(PACKAGE_FILES)} package files under {out.name}")
    return 0


def main() -> int:
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reproduce-to", metavar="SUBDIRECTORY")
    parser.add_argument("--write", action="store_true", help="regenerate CASES.json and the report")
    args = parser.parse_args()

    if args.reproduce_to is not None:
        return reproduce(args.reproduce_to)

    if args.write:
        report_lines = run_selftests()
        cases = build_cases()
        (PACKAGE_ROOT / "CASES.json").write_bytes(json_bytes(cases))
        report_lines.append(f"PASS case_count={len(cases)}")
        report_path = PACKAGE_ROOT / "selftestreport.txt"
        report_path.write_text("\n".join(report_lines) + "\n", encoding="utf-8")
        for line in report_lines:
            print(line)
        return 0

    ok, report = self_test()
    print(report, end="")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
