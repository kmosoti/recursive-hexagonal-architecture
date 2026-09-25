#!/usr/bin/env python3
"""Independent JSON-renderer corpus and reference projector.

The projector consumes the source-model representation described by the
contract.  It does not import, execute, or otherwise consult Rust code.
Running without arguments performs read-only self-tests.  Writing is only
performed by an explicit ``--reproduce-to`` invocation.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import random
import sys
from pathlib import Path
from typing import Any


SEED = 8072026
RANDOM_PROJECT_COUNT = 64
FIRST_GENERATION_CASES_SHA256 = "fb7b1f6f9bf28d6e7eda3cc0d17b2613c6ba0070166468df89d05fc43059e854"
PACKAGE_DIR = Path(__file__).resolve().parent
ALLOWED_OUTPUT_ROOT = PACKAGE_DIR
SNAPSHOT_DIR = PACKAGE_DIR / "source-snapshots"
SNAPSHOTS = {
    "contract.md": "docs/architecture/json-renderer-contract.md",
    "node.rs": "crates/document/src/node.rs",
    "page-model.rs": "crates/site/src/assembly/mod.rs",
    "original-prompt.md": "xtask/tests/corpus/write-prompts/CHG-008/pd-json-generator-prompt.md",
    "correction-prompt.md": "xtask/tests/corpus/write-prompts/CHG-008/pd-json-generator-correction-prompt.md",
    "whitespace-correction-prompt.md": "xtask/tests/corpus/write-prompts/CHG-008/pd-json-whitespace-correction-prompt.md",
}

CALL_OUTS = ("Note", "Tip", "Important", "Warning", "Caution")
ALIGNMENTS = ("None", "Left", "Center", "Right")
# Rust's str::split_whitespace uses Unicode White_Space, not Python's wider
# str.isspace classification (which also treats U+001C..U+001F as spaces).
UNICODE_WHITE_SPACE_RANGES = (
    (0x0009, 0x000D),
    (0x0020, 0x0020),
    (0x0085, 0x0085),
    (0x00A0, 0x00A0),
    (0x1680, 0x1680),
    (0x2000, 0x200A),
    (0x2028, 0x2029),
    (0x202F, 0x202F),
    (0x205F, 0x205F),
    (0x3000, 0x3000),
)
NODE_KINDS = (
    "Heading",
    "Paragraph",
    "Text",
    "Code",
    "CodeBlock",
    "Emphasis",
    "Strong",
    "Strikethrough",
    "Link",
    "WikiLink",
    "Image",
    "List",
    "BlockQuote",
    "Table",
    "Rule",
    "SoftBreak",
    "HardBreak",
    "Html",
    "TaskMarker",
)


class DuplicatePageId(Exception):
    """Reference equivalent of the constructor's only error."""

    def __init__(self, page_id: str) -> None:
        self.page_id = page_id
        super().__init__(page_id)


def _node(kind: str, **fields: Any) -> dict[str, Any]:
    result = {"kind": kind}
    result.update(fields)
    return result


def project_node(source: dict[str, Any]) -> dict[str, Any]:
    """Project one source-model node to its contract JSON object."""

    kind = source["kind"]
    if kind == "Heading":
        return {"type": "heading", "level": source["level"], "anchor": source["slug"], "children": [project_node(n) for n in source["children"]]}
    if kind in ("Paragraph", "Emphasis", "Strong", "Strikethrough"):
        return {
            "type": {
                "Paragraph": "paragraph",
                "Emphasis": "emphasis",
                "Strong": "strong",
                "Strikethrough": "strikethrough",
            }[kind],
            "children": [project_node(n) for n in source["children"]],
        }
    if kind in ("Text", "Code", "Html"):
        return {"type": {"Text": "text", "Code": "code", "Html": "html"}[kind], "text": source["text"]}
    if kind == "CodeBlock":
        return {"type": "code_block", "language": source["lang"], "text": source["text"]}
    if kind == "Link":
        return {"type": "link", "href": source["href"], "children": [project_node(n) for n in source["children"]]}
    if kind == "WikiLink":
        return {"type": "wiki_link", "link_index": source["index"], "children": [project_node(n) for n in source["children"]]}
    if kind == "Image":
        return {"type": "image", "src": source["src"], "alt": source["alt"]}
    if kind == "List":
        return {
            "type": "list",
            "start": source["start"],
            "items": [[project_node(n) for n in item] for item in source["items"]],
        }
    if kind == "BlockQuote":
        callout = source["callout"]
        return {
            "type": "block_quote",
            "kind": None if callout is None else callout.lower(),
            "children": [project_node(n) for n in source["children"]],
        }
    if kind == "Table":
        return {
            "type": "table",
            "align": [alignment.lower() for alignment in source["align"]],
            "head": [[project_node(n) for n in cell] for cell in source["head"]],
            "rows": [[[project_node(n) for n in cell] for cell in row] for row in source["rows"]],
        }
    if kind == "TaskMarker":
        return {"type": "task_marker", "checked": source["checked"]}
    if kind in ("Rule", "SoftBreak", "HardBreak"):
        return {"type": {"Rule": "rule", "SoftBreak": "soft_break", "HardBreak": "hard_break"}[kind]}
    raise ValueError(f"unknown source node kind: {kind}")


def _flatten_parts(source: dict[str, Any]) -> list[str]:
    """Collect visible text, using explicit boundaries from the contract."""

    kind = source["kind"]
    if kind in ("Text", "Code", "Html"):
        return [source["text"]]
    if kind == "Image":
        return [source["alt"]]
    if kind in ("Link", "WikiLink", "Emphasis", "Strong", "Strikethrough"):
        return [part for child in source["children"] for part in _flatten_parts(child)]
    if kind in ("Heading", "Paragraph", "BlockQuote"):
        return [" "] + [part for child in source["children"] for part in _flatten_parts(child)] + [" "]
    if kind == "CodeBlock":
        return [" ", source["text"], " "]
    if kind == "List":
        parts: list[str] = []
        for item in source["items"]:
            parts.extend([" "])
            for child in item:
                parts.extend(_flatten_parts(child))
            parts.extend([" "])
        return parts
    if kind == "Table":
        parts = []
        cells = list(source["head"])
        cells.extend(cell for row in source["rows"] for cell in row)
        for cell in cells:
            parts.append(" ")
            for child in cell:
                parts.extend(_flatten_parts(child))
            parts.append(" ")
        return parts
    if kind in ("SoftBreak", "HardBreak", "Rule"):
        return [" "]
    if kind == "TaskMarker":
        return []
    raise ValueError(f"unknown source node kind: {kind}")


def _is_unicode_white_space(character: str) -> bool:
    codepoint = ord(character)
    return any(start <= codepoint <= end for start, end in UNICODE_WHITE_SPACE_RANGES)


def _split_contract_whitespace(value: str) -> list[str]:
    fields: list[str] = []
    field: list[str] = []
    for character in value:
        if _is_unicode_white_space(character):
            if field:
                fields.append("".join(field))
                field = []
        else:
            field.append(character)
    if field:
        fields.append("".join(field))
    return fields


def flatten_text(body: list[dict[str, Any]]) -> str:
    """Return normalized search text for a page body."""

    raw = "".join(part for node in body for part in _flatten_parts(node))
    return " ".join(_split_contract_whitespace(raw))


def project_page(source: dict[str, Any]) -> dict[str, Any]:
    links = []
    for link in source["links"]:
        if link["kind"] == "Page":
            links.append({"status": "resolved", "page": link["id"], "anchor": link["anchor"]})
        elif link["kind"] == "Unresolved":
            links.append({"status": "unresolved", "page": None, "anchor": None})
        else:
            raise ValueError(f"unknown link kind: {link['kind']}")
    return {
        "schema_version": 1,
        "kind": "rhawiki_page",
        "id": source["id"],
        "title": source["title"],
        "toc": [dict(entry) for entry in source["toc"]],
        "breadcrumbs": list(source["breadcrumbs"]),
        "backlinks": [dict(entry) for entry in source["backlinks"]],
        "links": links,
        "body": [project_node(node) for node in source["body"]],
        "built_at": source["built_at"],
    }


def project_index(pages: list[dict[str, Any]]) -> dict[str, Any]:
    ids = [page["id"] for page in pages]
    if len(ids) != len(set(ids)):
        seen: set[str] = set()
        for page_id in ids:
            if page_id in seen:
                raise DuplicatePageId(page_id)
            seen.add(page_id)
    entries = []
    for page in sorted(pages, key=lambda item: item["id"]):
        entries.append(
            {
                "id": page["id"],
                "path": f"{page['id']}.json",
                "title": page["title"],
                "headings": [dict(entry) for entry in page["toc"]],
                "text": flatten_text(page["body"]),
            }
        )
    return {"schema_version": 1, "kind": "rhawiki_search_index", "entries": entries}


def project_pages(pages: list[dict[str, Any]]) -> tuple[dict[str, Any], dict[str, Any]]:
    """Project pages and the immutable index, raising on duplicate IDs."""

    index = project_index(pages)
    expected_pages = {f"{page['id']}.json": project_page(page) for page in pages}
    return expected_pages, index


def _page(
    page_id: str,
    title: str,
    *,
    toc: list[dict[str, Any]] | None = None,
    breadcrumbs: list[str] | None = None,
    backlinks: list[dict[str, str]] | None = None,
    links: list[dict[str, Any]] | None = None,
    body: list[dict[str, Any]] | None = None,
    built_at: str | None = None,
) -> dict[str, Any]:
    return {
        "id": page_id,
        "title": title,
        "toc": [] if toc is None else toc,
        "breadcrumbs": [] if breadcrumbs is None else breadcrumbs,
        "backlinks": [] if backlinks is None else backlinks,
        "links": [] if links is None else links,
        "body": [] if body is None else body,
        "built_at": built_at,
    }


def _source(path: str, text: str) -> dict[str, str]:
    return {"path": path, "text": text}


def _all_variant_page() -> dict[str, Any]:
    links = [
        {"kind": "Page", "id": "unicode/target", "anchor": "résumé?x=1"},
        {"kind": "Unresolved"},
    ]
    inline = [
        _node("Text", text="inline"),
        _node("Code", text="`literal`"),
        _node("Html", text='<span data-x="&">raw</span>'),
        _node("Link", href='https://example.test/p?q=one&x=2\\path"quote', children=[_node("Text", text="href-label")]),
        _node("WikiLink", index=0, children=[_node("Text", text="wiki-label")]),
        _node("Image", src='img\\name?q="x"', alt="image alt"),
    ]
    body = [
        _node("Heading", level=2, slug="body-heading", children=[_node("Text", text="Body heading")]),
        _node("Paragraph", children=inline),
        _node("Emphasis", children=[_node("Strong", children=[_node("Text", text="nested emphasis")])]),
        _node("Strikethrough", children=[_node("Text", text="gone")]),
        _node("CodeBlock", lang=None, text="code block\nraw"),
        _node("CodeBlock", lang="rust", text='fn main() { println!("x"); }'),
        _node("List", start=None, items=[[], [_node("Text", text="item two")]]),
        _node("List", start=7, items=[[_node("Text", text="seven")], [_node("Paragraph", children=[_node("Text", text="nested item")])]]),
        *[
            _node("BlockQuote", callout=callout, children=[_node("Text", text="plain" if callout is None else callout)])
            for callout in [None, *CALL_OUTS]
        ],
        _node(
            "Table",
            align=list(ALIGNMENTS),
            head=[[_node("Text", text="head-1")], [_node("Text", text="head-2")]],
            rows=[
                [[_node("Text", text="left")], [_node("Text", text="center")], [_node("Text", text="right")], []],
                [[_node("Code", text="row2")], [_node("Html", text="<b>row2</b>")]],
            ],
        ),
        _node("TaskMarker", checked=False),
        _node("TaskMarker", checked=True),
        _node("Rule"),
        _node("SoftBreak"),
        _node("HardBreak"),
    ]
    return _page(
        "unicode/complete",
        "Títle 日本 😀",
        toc=[
            {"level": 1, "text": "TOC differs", "anchor": "toc-diff"},
            {"level": 3, "text": "Second", "anchor": "second"},
        ],
        breadcrumbs=["unicode", "fixtures"],
        backlinks=[{"id": "z-last", "title": "Last"}, {"id": "a-first", "title": "First"}],
        links=links,
        body=body,
        built_at="2026-09-23T00:00:00Z",
    )


def _fixed_project_cases() -> list[dict[str, Any]]:
    complete = _all_variant_page()
    empty = _page("empty", "Empty")
    inline = _page(
        "inline",
        "Inline",
        links=[{"kind": "Unresolved"}],
        body=[
            _node(
                "Paragraph",
                children=[
                    _node("Text", text="a"),
                    _node("Code", text="b"),
                    _node("Html", text="c"),
                    _node("Link", href="/literal?x=1&y=2", children=[_node("Text", text="d")]),
                    _node("WikiLink", index=0, children=[_node("Text", text="e")]),
                    _node("Image", src="image.png", alt="f"),
                    _node("Emphasis", children=[_node("Text", text="g")]),
                ],
            ),
            _node("Paragraph", children=[_node("Text", text="block-one")]),
            _node("Paragraph", children=[_node("Text", text="block-two")]),
            _node("List", start=1, items=[[_node("Text", text="one")], [_node("Text", text="two")]]),
            _node("Table", align=["None"], head=[[_node("Text", text="cell-a")]], rows=[[[ _node("Text", text="cell-b") ]]]),
        ],
    )
    whitespace = _page(
        "whitespace",
        "Whitespace",
        body=[
            _node("Paragraph", children=[_node("Text", text="alpha\u00a0\tbeta\ngamma")]),
            _node("Paragraph", children=[_node("Text", text="delta")]),
            _node("CodeBlock", lang="text", text="epsilon\u2003zeta"),
            _node("SoftBreak"),
            _node("Text", text="eta\n\n theta"),
        ],
    )
    ordered_a = _page(
        "a-first",
        "A first",
        toc=[{"level": 1, "text": "A", "anchor": "a"}],
        body=[_node("Heading", level=1, slug="a", children=[_node("Text", text="A")])],
        built_at=None,
    )
    ordered_z = _page(
        "z-last",
        "Z last",
        toc=[{"level": 1, "text": "Z", "anchor": "z"}],
        body=[_node("Heading", level=1, slug="z", children=[_node("Text", text="Z")])],
        built_at="t0",
    )
    pages = [complete, empty, inline, whitespace]
    expected_pages, expected_index = project_pages(pages)
    cases = [
        {"id": "project-all-node-variants", "kind": "project", "pages": pages, "expected_pages": expected_pages, "expected_index": expected_index},
    ]
    for order_name, ordered in (("forward", [ordered_a, ordered_z]), ("reversed", [ordered_z, ordered_a])):
        expected_pages, expected_index = project_pages(ordered)
        case = {"id": f"project-ordering-{order_name}", "kind": "project", "pages": ordered, "expected_pages": expected_pages, "expected_index": expected_index}
        if order_name == "reversed":
            forward_pages, forward_index = project_pages([ordered_a, ordered_z])
            case["comparison_pages"] = [ordered_a, ordered_z]
            case["expected_comparison_pages"] = forward_pages
            case["expected_comparison_index"] = forward_index
        cases.append(case)
    expected_pages, expected_index = project_pages([inline])
    cases.append({"id": "project-inline-and-block-boundaries", "kind": "project", "pages": [inline], "expected_pages": expected_pages, "expected_index": expected_index})
    expected_pages, expected_index = project_pages([whitespace])
    cases.append({"id": "project-unicode-whitespace", "kind": "project", "pages": [whitespace], "expected_pages": expected_pages, "expected_index": expected_index})
    clock_t1 = dict(ordered_a, built_at="2026-09-23T01:02:03Z")
    clock_t2 = dict(ordered_a, built_at="2027-01-01T00:00:00Z")
    expected_pages, expected_index = project_pages([clock_t1])
    _, expected_clock_index = project_pages([clock_t2])
    cases.append(
        {
            "id": "project-clock-independent-index",
            "kind": "project",
            "pages": [clock_t1],
            "expected_pages": expected_pages,
            "expected_index": expected_index,
            "comparison_pages": [clock_t2],
            "expected_comparison_index": expected_clock_index,
        }
    )
    return cases


def _random_string(rng: random.Random, label: str) -> str:
    choices = [
        f"{label}-{rng.randrange(1000)}",
        "café\u00a0tab\tline\nnext",
        "<em>raw & html</em>",
        "quote \\\" and \\ slash",
        "日本語 😀",
    ]
    return rng.choice(choices)


def _random_leaf(rng: random.Random, link_count: int) -> dict[str, Any]:
    kind = rng.choice(("Text", "Code", "Html", "Image", "TaskMarker", "Rule", "SoftBreak", "HardBreak", "WikiLink" if link_count else "Text"))
    if kind in ("Text", "Code", "Html"):
        return _node(kind, text=_random_string(rng, kind.lower()))
    if kind == "Image":
        return _node("Image", src=f"assets/{rng.randrange(4)}.png", alt=_random_string(rng, "alt"))
    if kind == "TaskMarker":
        return _node("TaskMarker", checked=bool(rng.randrange(2)))
    if kind == "WikiLink":
        return _node("WikiLink", index=rng.randrange(link_count), children=[_random_leaf(rng, 0)])
    return _node(kind)


def _random_node(rng: random.Random, depth: int, link_count: int) -> dict[str, Any]:
    if depth >= 3:
        return _random_leaf(rng, link_count)
    kind = rng.choice(NODE_KINDS)
    if kind in ("Text", "Code", "Html", "Image", "TaskMarker", "Rule", "SoftBreak", "HardBreak"):
        return _random_leaf(rng, link_count)
    if kind == "Heading":
        return _node("Heading", level=rng.randrange(1, 5), slug=f"h-{rng.randrange(100)}", children=[_random_node(rng, depth + 1, link_count)])
    if kind in ("Paragraph", "Emphasis", "Strong", "Strikethrough"):
        count = rng.randrange(0, 3)
        return _node(kind, children=[_random_node(rng, depth + 1, link_count) for _ in range(count)])
    if kind == "CodeBlock":
        return _node("CodeBlock", lang=rng.choice([None, "text", "rust"]), text=_random_string(rng, "block"))
    if kind in ("Link", "WikiLink"):
        if kind == "WikiLink" and not link_count:
            return _random_leaf(rng, 0)
        fields: dict[str, Any] = {"children": [_random_node(rng, depth + 1, link_count)]}
        if kind == "Link":
            fields["href"] = rng.choice(["/page?q=1&x=2", "https://example.test/a\\b"])
        else:
            fields["index"] = rng.randrange(link_count)
        return _node(kind, **fields)
    if kind == "List":
        items = []
        for _ in range(rng.randrange(0, 3)):
            items.append([_random_node(rng, depth + 1, link_count) for _ in range(rng.randrange(0, 3))])
        return _node("List", start=rng.choice([None, 1, 3, 42]), items=items)
    if kind == "BlockQuote":
        return _node("BlockQuote", callout=rng.choice([None, *CALL_OUTS]), children=[_random_node(rng, depth + 1, link_count)])
    if kind == "Table":
        def cell() -> list[dict[str, Any]]:
            return [_random_node(rng, depth + 1, link_count) for _ in range(rng.randrange(0, 2))]

        return _node(
            "Table",
            align=[rng.choice(ALIGNMENTS) for _ in range(rng.randrange(0, 4))],
            head=[cell() for _ in range(rng.randrange(0, 3))],
            rows=[[cell() for _ in range(rng.randrange(0, 3))] for _ in range(rng.randrange(0, 3))],
        )
    raise AssertionError(kind)


def _random_project_case(rng: random.Random, ordinal: int) -> dict[str, Any]:
    count = rng.randrange(1, 4)
    ids = [f"random-{ordinal:02d}-page-{index}" for index in range(count)]
    pages = []
    for index, page_id in enumerate(ids):
        link_count = rng.randrange(0, 3)
        links = []
        for link_index in range(link_count):
            if rng.randrange(3) == 0:
                links.append({"kind": "Unresolved"})
            else:
                links.append({"kind": "Page", "id": rng.choice(ids), "anchor": rng.choice([None, f"section-{link_index}"])})
        toc = [
            {"level": rng.randrange(1, 5), "text": _random_string(rng, "toc"), "anchor": f"toc-{index}-{toc_index}"}
            for toc_index in range(rng.randrange(0, 4))
        ]
        body = [_random_node(rng, 0, link_count) for _ in range(rng.randrange(1, 5))]
        pages.append(
            _page(
                page_id,
                _random_string(rng, "title"),
                toc=toc,
                breadcrumbs=[f"section-{index}"] if rng.randrange(2) else [],
                backlinks=[{"id": rng.choice(ids), "title": _random_string(rng, "backlink")} for _ in range(rng.randrange(0, 3))],
                links=links,
                body=body,
                built_at=rng.choice([None, "2026-09-23T00:00:00Z", "t0"]),
            )
        )
    if rng.randrange(2):
        rng.shuffle(pages)
    expected_pages, expected_index = project_pages(pages)
    return {"id": f"project-random-{ordinal:02d}", "kind": "project", "pages": pages, "expected_pages": expected_pages, "expected_index": expected_index}


def _flow_cases() -> list[dict[str, Any]]:
    representative = [_all_variant_page()]
    return [
        {
            "id": "duplicate-page-id-forward",
            "kind": "duplicate",
            "pages": [_page("dup", "First"), _page("unique", "Unique"), _page("dup", "Second")],
            "expected_error": {"kind": "DuplicatePageId", "id": "dup"},
        },
        {
            "id": "duplicate-page-id-reversed",
            "kind": "duplicate",
            "pages": [_page("dup", "Second"), _page("unique", "Unique"), _page("dup", "First")],
            "expected_error": {"kind": "DuplicatePageId", "id": "dup"},
        },
        {
            "id": "reconcile-delete-page",
            "kind": "reconcile",
            "before_sources": [_source("A.md", "# A\n"), _source("B.md", "# B\n")],
            "after_sources": [_source("A.md", "# A\n")],
            "clock": "t0",
            "clock_type": "FixedClock",
            "fresh_renderer_each_build": True,
            "expected_initial_paths": ["A.json", "B.json", "assets/search-index.json"],
            "expected_final_paths": ["A.json", "assets/search-index.json"],
            "expected_deleted_paths": ["B.json"],
            "expected_index_ids": ["A"],
            "expected_unchanged": 1,
        },
        {
            "id": "collision-before-writes",
            "kind": "collision",
            "sources": [_source("assets/search-index.md", "# Search index\n")],
            "expected_error_contains": "Inv_K: two writes to assets/search-index.json",
            "expected_sink_files": 0,
        },
        {
            "id": "owner-contract-representative",
            "kind": "contract",
            "pages": representative,
            "expected_violations": [],
            "owner_contract_passed": True,
        },
        {
            "id": "cli-json-basic-and-nested",
            "kind": "cli_json",
            "sources": [_source("A.md", "# A\n"), _source("nested/B.md", "# B\n")],
            "expected_paths": ["A.json", "assets/search-index.json", "nested/B.json"],
            "expected_index_ids": ["A", "nested/B"],
            "expected_titles": {"A": "A", "nested/B": "B"},
            "expected_page_schema": {"schema_version": 1, "kind": "rhawiki_page"},
            "required_page_members": ["schema_version", "kind", "title", "toc"],
            "expected_toc": {
                "A": [{"level": 1, "text": "A", "anchor": "a"}],
                "nested/B": [{"level": 1, "text": "B", "anchor": "b"}],
            },
            "expected_exit": 0,
        },
        {
            "id": "cli-html-default",
            "kind": "cli_html",
            "sources": [_source("A.md", "# A\n")],
            "explicit_format": False,
            "expected_paths": ["A.html", "assets/style.css"],
            "expected_exit": 0,
            "expected_markers": {"A.html": ["A", "<h1"]},
        },
        {
            "id": "cli-html-explicit",
            "kind": "cli_html",
            "sources": [_source("A.md", "# A\n")],
            "explicit_format": True,
            "expected_paths": ["A.html", "assets/style.css"],
            "expected_exit": 0,
            "expected_markers": {"A.html": ["A", "<h1"]},
        },
        {
            "id": "cli-check-json-v1",
            "kind": "cli_check",
            "sources": [_source("A.md", "# A\n")],
            "expected_exit": 0,
            "expected_json": {"schema_version": 1, "pages": 1, "witnesses": [], "counts": {}},
        },
    ]


def make_cases(seed: int = SEED) -> list[dict[str, Any]]:
    if seed != SEED:
        raise ValueError(f"corpus seed is fixed at {SEED}")
    cases = _fixed_project_cases()
    rng = random.Random(seed)
    cases.extend(_random_project_case(rng, ordinal) for ordinal in range(RANDOM_PROJECT_COUNT))
    cases.extend(_flow_cases())
    return cases


def _json_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2, separators=(",", ": ")) + "\n").encode("utf-8")


def _sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _sha256_path(path: Path) -> str:
    return _sha256_bytes(path.read_bytes())


def _kind_counts(cases: list[dict[str, Any]]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for case in cases:
        counts[case["kind"]] = counts.get(case["kind"], 0) + 1
    return dict(sorted(counts.items()))


def _assert_source_coverage(cases: list[dict[str, Any]]) -> None:
    source_nodes: list[dict[str, Any]] = []

    def visit(node: dict[str, Any]) -> None:
        source_nodes.append(node)
        kind = node["kind"]
        for child in node.get("children", []):
            visit(child)
        for item in node.get("items", []):
            for child in item:
                visit(child)
        for cell in node.get("head", []):
            for child in cell:
                visit(child)
        for row in node.get("rows", []):
            for cell in row:
                for child in cell:
                    visit(child)
        if kind not in NODE_KINDS:
            raise AssertionError(kind)

    project_cases = [case for case in cases if case["kind"] == "project"]
    for case in project_cases:
        for page in case["pages"]:
            page_start = len(source_nodes)
            for node in page["body"]:
                visit(node)
            for node in source_nodes[page_start:]:
                if node["kind"] == "WikiLink":
                    assert 0 <= node["index"] < len(page["links"])
    kinds = {node["kind"] for node in source_nodes}
    missing = set(NODE_KINDS) - kinds
    assert not missing, f"missing node variants: {sorted(missing)}"
    assert {node["callout"] for node in source_nodes if node["kind"] == "BlockQuote"} >= {None, *CALL_OUTS}
    assert {alignment for node in source_nodes if node["kind"] == "Table" for alignment in node["align"]} >= set(ALIGNMENTS)
    assert {node["lang"] for node in source_nodes if node["kind"] == "CodeBlock"} >= {None, "rust"}
    assert {node["start"] for node in source_nodes if node["kind"] == "List"} >= {None, 7}
    assert {page["built_at"] for case in project_cases for page in case["pages"]} >= {None, "2026-09-23T00:00:00Z", "t0"}
    assert any(page["toc"] and page["body"] and page["toc"][0]["text"] != "Body heading" for case in project_cases for page in case["pages"])
    assert any("\u00a0" in node.get("text", "") for node in source_nodes)
    assert any(node["kind"] == "Html" and "<span" in node["text"] for node in source_nodes)
    assert any(node["kind"] == "Link" and "?" in node["href"] and "\\" in node["href"] and '"' in node["href"] for node in source_nodes)
def _self_test_values(cases: list[dict[str, Any]]) -> list[str]:
    assert len(cases) == RANDOM_PROJECT_COUNT + 6 + 9
    assert sum(1 for case in cases if case["kind"] == "project") == RANDOM_PROJECT_COUNT + 6
    assert _kind_counts(cases) == {"cli_check": 1, "cli_html": 2, "cli_json": 1, "collision": 1, "contract": 1, "duplicate": 2, "project": 70, "reconcile": 1}
    _assert_source_coverage(cases)

    assert flatten_text([_node("Paragraph", children=[_node("Text", text="a"), _node("Code", text="b"), _node("Html", text="c")])]) == "abc"
    assert flatten_text([_node("Paragraph", children=[_node("Text", text="a")]), _node("Paragraph", children=[_node("Text", text="b")])]) == "a b"
    assert flatten_text([_node("Text", text="a\u00a0\tb\n c")]) == "a b c"
    assert flatten_text([_node("TaskMarker", checked=True), _node("Text", text="visible")]) == "visible"
    assert flatten_text([_node("Link", href="/target", children=[_node("Text", text="label")])]) == "label"
    assert flatten_text([_node("Image", src="image", alt="alt")]) == "alt"
    for codepoint in range(0x001C, 0x0020):
        assert flatten_text([_node("Text", text=f"A{chr(codepoint)}B")]) == f"A{chr(codepoint)}B"
    for codepoint in (0x0085, 0x00A0, 0x2003):
        assert flatten_text([_node("Text", text=f"A{chr(codepoint)}B")]) == "A B"
    complete_whitespace = "".join(chr(codepoint) for start, end in UNICODE_WHITE_SPACE_RANGES for codepoint in range(start, end + 1))
    assert _split_contract_whitespace(f"left{complete_whitespace}right") == ["left", "right"]

    all_case = next(case for case in cases if case["id"] == "project-all-node-variants")
    complete_text = next(entry["text"] for entry in all_case["expected_index"]["entries"] if entry["id"] == "unicode/complete")
    assert complete_text == "Body heading inline`literal`<span data-x=\"&\">raw</span>href-labelwiki-labelimage alt nested emphasisgone code block raw fn main() { println!(\"x\"); } item two seven nested item plain Note Tip Important Warning Caution head-1 head-2 left center right row2 <b>row2</b>"
    assert all_case["expected_pages"]["unicode/complete.json"]["built_at"] == "2026-09-23T00:00:00Z"
    clock = next(case for case in cases if case["id"] == "project-clock-independent-index")
    assert clock["expected_index"] == clock["expected_comparison_index"]
    assert next(case for case in cases if case["id"] == "project-ordering-reversed")["expected_index"] == next(case for case in cases if case["id"] == "project-ordering-forward")["expected_index"]
    duplicate = next(case for case in cases if case["kind"] == "duplicate")
    try:
        project_pages(duplicate["pages"])
    except DuplicatePageId as error:
        assert error.page_id == "dup"
    else:
        raise AssertionError("duplicate control did not reject")

    encoded_once = _json_bytes(cases)
    encoded_twice = _json_bytes(make_cases(SEED))
    assert encoded_once == encoded_twice
    assert _sha256_bytes(encoded_once) == FIRST_GENERATION_CASES_SHA256
    for snapshot_name in SNAPSHOTS:
        assert (SNAPSHOT_DIR / snapshot_name).is_file(), snapshot_name
    return [
        "projector projection controls: passed",
        "hand-derived flatten controls: passed",
        "duplicate constructor control: passed",
        "seeded reproduction byte determinism: passed",
        "Rust White_Space normalization controls: passed",
        "first-generation CASES.json identity: passed",
        "frozen source/prompt snapshot closure: passed",
        f"case count: {len(cases)}",
        f"kind counts: {_kind_counts(cases)}",
    ]


def _readme(cases: list[dict[str, Any]]) -> str:
    counts = _kind_counts(cases)
    return f"""# JSON renderer corpus

This package is an independent source-model fixture set and Python reference
projector for JSON Renderer Contract v1. It does not use a Rust encoder or
claim that an adapter implementation passes.

- Fixed seed: `{SEED}` via `random.Random`
- Seeded random project cases: `{RANDOM_PROJECT_COUNT}`
- Total cases: `{len(cases)}`
- Case kinds: `{json.dumps(counts, sort_keys=True)}`

Coverage includes every source `Node` variant; all four table alignments; all
five callout kinds plus null; null and non-null code-block languages and list
starts; nested containers; resolved and unresolved links; empty vectors;
Unicode, Unicode whitespace, raw HTML text, literal query/backslash/quote
hrefs; independent TOC data; source ordering and reversed input; adjacent
inline fragments; distinct block, list, and table-cell boundaries; and index
clock independence.

The explicit flow cases cover duplicate IDs in two orderings, fresh-renderer
reconciliation with deletion, pre-write page/asset collision, the owner
contract controls, JSON CLI selection, HTML default and explicit selection,
and unchanged JSON check output. Their expected fields are stored alongside
their source inputs in `CASES.json`.

## Reference commands

`python3 reference.py` runs read-only self-tests. From the repository root,
copy the committed package to a fresh, owned staging directory before
reproducing:

`cp -R xtask/tests/corpus/json-renderer target/m2/json-renderer-reproduction`

`python3 target/m2/json-renderer-reproduction/reference.py --reproduce-to target/m2/json-renderer-reproduction/reproduced`

The staging destination must be fresh. The reproduction writes only under the
staged package, and its `reproduced/` subdirectory may be removed afterward.
The `source-snapshots/` directory freezes the contract, model vocabulary, and
all three durable prompt archives. Reproduction reads those snapshots, never
future live source files, and copies the snapshots into the explicit
reproduction subdirectory. Self-tests from the committed script are
read-only.

JSON expected values are decoded objects in the case stream; deterministic
pretty UTF-8 output with one final newline is a separate renderer obligation.
"""


def _toml_string(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def _registration(cases: list[dict[str, Any]], payloads: dict[str, bytes], sums: bytes) -> str:
    source_specs = [
        ("contract", "xtask/tests/corpus/json-renderer/source-snapshots/contract.md", SNAPSHOTS["contract.md"], "contract.md"),
        ("node", "xtask/tests/corpus/json-renderer/source-snapshots/node.rs", SNAPSHOTS["node.rs"], "node.rs"),
        ("page_model", "xtask/tests/corpus/json-renderer/source-snapshots/page-model.rs", SNAPSHOTS["page-model.rs"], "page-model.rs"),
    ]
    original_prompt_path = "xtask/tests/corpus/write-prompts/CHG-008/pd-json-generator-prompt.md"
    correction_prompt_path = "xtask/tests/corpus/write-prompts/CHG-008/pd-json-generator-correction-prompt.md"
    whitespace_prompt_path = "xtask/tests/corpus/write-prompts/CHG-008/pd-json-whitespace-correction-prompt.md"
    original_prompt_hash = _sha256_path(SNAPSHOT_DIR / "original-prompt.md")
    correction_prompt_hash = _sha256_path(SNAPSHOT_DIR / "correction-prompt.md")
    whitespace_prompt_hash = _sha256_path(SNAPSHOT_DIR / "whitespace-correction-prompt.md")
    lines = [
        "schema_version = 1",
        f"seed = {SEED}",
        'model = "gpt-5.6-luna"',
        'effort = "high"',
        f"prompt_path = {_toml_string(original_prompt_path)}",
        f"prompt_sha256 = {_toml_string(original_prompt_hash)}",
        f"correction_prompt_path = {_toml_string(correction_prompt_path)}",
        f"correction_prompt_sha256 = {_toml_string(correction_prompt_hash)}",
        f"whitespace_prompt_path = {_toml_string(whitespace_prompt_path)}",
        f"whitespace_prompt_sha256 = {_toml_string(whitespace_prompt_hash)}",
        "prompt_count = 3",
        f"case_count = {len(cases)}",
        f"random_project_count = {RANDOM_PROJECT_COUNT}",
        f"payload_count = {len(payloads)}",
        f"content_sha256 = {_toml_string(_sha256_bytes(sums))}",
        "",
        "[kind_counts]",
    ]
    for kind, count in _kind_counts(cases).items():
        lines.append(f"{kind} = {count}")
    lines.extend(["", "[[sources]]"])
    for index, (name, durable_path, original_path, snapshot_name) in enumerate(source_specs):
        if index:
            lines.append("\n[[sources]]")
        lines.append(f"name = {_toml_string(name)}")
        lines.append(f"source_path = {_toml_string(durable_path)}")
        lines.append(f"original_path = {_toml_string(original_path)}")
        lines.append(f"source_sha256 = {_toml_string(_sha256_path(SNAPSHOT_DIR / snapshot_name))}")
    lines.extend(["", "[[inventory]]"])
    for index, (path, data) in enumerate(sorted(payloads.items())):
        if index:
            lines.append("\n[[inventory]]")
        lines.append(f"path = {_toml_string(path)}")
        lines.append(f"bytes = {len(data)}")
        lines.append(f"sha256 = {_toml_string(_sha256_bytes(data))}")
    lines.extend(
        [
            "",
            "[self_tests]",
            "reference_projector = true",
            "hand_derived_flatten_controls = true",
            "duplicate_id_control = true",
            "seeded_reproduction_byte_check = true",
            "adapter_implementation = \"not_run\"",
            "",
        ]
    )
    return "\n".join(lines)


def _write_package(destination: Path, cases: list[dict[str, Any]], report: list[str]) -> None:
    destination = destination.resolve()
    try:
        destination.relative_to(ALLOWED_OUTPUT_ROOT.resolve())
    except ValueError as error:
        raise ValueError(f"refusing to write outside {ALLOWED_OUTPUT_ROOT}") from error
    destination.mkdir(parents=True, exist_ok=True)

    payloads: dict[str, bytes] = {
        "CASES.json": _json_bytes(cases),
        "README.md": _readme(cases).encode("utf-8"),
        "reference.py": Path(__file__).read_bytes(),
        "selftestreport.txt": ("Independent reference self-test report\n" + "\n".join(report) + "\n").encode("utf-8"),
    }
    payloads.update({f"source-snapshots/{name}": (SNAPSHOT_DIR / name).read_bytes() for name in sorted(SNAPSHOTS)})
    assert _sha256_bytes(payloads["CASES.json"]) == FIRST_GENERATION_CASES_SHA256
    for name, data in payloads.items():
        output = destination / name
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(data)

    sum_lines = [f"{_sha256_bytes(payloads[name])}  {name}" for name in sorted(payloads)]
    sums = ("\n".join(sum_lines) + "\n").encode("utf-8")
    (destination / "SHA256SUMS").write_bytes(sums)
    registration = _registration(cases, payloads, sums).encode("utf-8")
    (destination / "registration.toml").write_bytes(registration)

    # The committed stream must be exactly the stream generated from the seed.
    assert (destination / "CASES.json").read_bytes() == _json_bytes(make_cases(SEED))


def self_test() -> list[str]:
    cases = make_cases(SEED)
    return _self_test_values(cases)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reproduce-to", type=Path, help="explicit output directory under target/m2")
    args = parser.parse_args(argv)
    report = self_test()
    if args.reproduce_to is not None:
        _write_package(args.reproduce_to, make_cases(SEED), report)
        print(f"reproduced {len(make_cases(SEED))} cases to {args.reproduce_to}")
    else:
        print("reference self-test passed")
        for line in report:
            print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
