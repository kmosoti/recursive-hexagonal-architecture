#!/usr/bin/env python3
"""Independent, deterministic author and validator for the site fixtures.

This is a fixture author/validator, not a complete Markdown or PageModel
reference implementation.  The expected observations are authored from the
committed transclusion contract and deliberately cover only the fields named
by the package schema.
"""

from __future__ import annotations

import argparse
import json
import shutil
import sys
from pathlib import Path
from typing import Any


PACKAGE = Path(__file__).resolve().parent
PACKAGE_FILES = (
    "CASES.json",
    "README.md",
    "SHA256SUMS",
    "reference.py",
    "registration.toml",
    "selftestreport.txt",
    "source-snapshots/bdr.md",
    "source-snapshots/contract.md",
    "source-snapshots/prompt.md",
)


def source(path: str, text: str) -> dict[str, str]:
    return {"path": path, "text": text}


def document(
    *,
    transclusions: list[dict[str, Any]] | None = None,
    navigation_links: list[dict[str, Any]] | None = None,
    images: list[dict[str, str]] | None = None,
    heading_slugs: list[str] | None = None,
) -> dict[str, Any]:
    return {
        "transclusions": transclusions or [],
        "navigation_links": navigation_links or [],
        "images": images or [],
        "heading_slugs": heading_slugs or [],
    }


def tx(number: int, target: str, anchor: str | None, display: str, line: int) -> dict[str, Any]:
    return {
        "id": number,
        "target": target,
        "anchor": anchor,
        "display": display,
        "line": line,
    }


def nav(target: str, anchor: str | None, alias: str | None, line: int) -> dict[str, Any]:
    return {"target": target, "anchor": anchor, "alias": alias, "line": line}


def image(src: str, alt: str) -> dict[str, str]:
    return {"src": src, "alt": alt}


def page(
    toc_anchors: list[str],
    heading_ids: list[str],
    wiki_targets: list[dict[str, str | None]],
    ordinary_hrefs: list[str],
    image_sources: list[str],
    text_includes: list[str],
    text_excludes: list[str],
) -> dict[str, Any]:
    return {
        "toc_anchors": toc_anchors,
        "heading_ids": heading_ids,
        "wiki_targets": wiki_targets,
        "ordinary_hrefs": ordinary_hrefs,
        "image_sources": image_sources,
        "text_includes": text_includes,
        "text_excludes": text_excludes,
    }


def check(witnesses: list[dict[str, Any]], counts: dict[str, int]) -> dict[str, Any]:
    return {"schema_version": 1, "pages": 0, "witnesses": witnesses, "counts": counts}


def row(
    ident: str,
    sources: list[dict[str, str]],
    expected_documents: dict[str, dict[str, Any]],
    expected_pages: dict[str, dict[str, Any]],
    witnesses: list[dict[str, Any]],
    counts: dict[str, int],
    expected_exit: int,
    **extra: Any,
) -> dict[str, Any]:
    expected = check(witnesses, counts)
    expected["pages"] = len(sources)
    result: dict[str, Any] = {
        "id": ident,
        "kind": "site",
        "sources": sources,
        "expected_documents": expected_documents,
        "expected_check": expected,
        "expected_exit": expected_exit,
        "expected_pages": expected_pages,
    }
    result.update(extra)
    return result


def cases() -> list[dict[str, Any]]:
    """Return the frozen twelve authored scenario groups in stable order."""

    return [
        row(
            "TX01",
            [
                source(
                    "tx01/host",
                    "# Host\n"
                    "before\n"
                    "\n"
                    "![[tx01/target]]\n"
                    "\n"
                    "after\n"
                    "\n"
                    "  ![[tx01/target#part]]  \n",
                ),
                source("tx01/target", "# Target\nintro\n## Part\npart\n"),
            ],
            {
                "tx01/host": document(
                    transclusions=[
                        tx(0, "tx01/target", None, "tx01/target", 4),
                        tx(1, "tx01/target", "part", "tx01/target#part", 8),
                    ],
                    heading_slugs=["host"],
                ),
                "tx01/target": document(heading_slugs=["target", "part"]),
            },
            {
                "tx01/host": page(
                    ["host"],
                    ["host", "tx-0-target", "tx-0-part", "tx-1-part"],
                    [],
                    [],
                    [],
                    ["before", "intro", "part", "after"],
                    [],
                )
            },
            [],
            {},
            0,
        ),
        row(
            "TX02",
            [
                source(
                    "tx02/host",
                    "# Exclusions\n"
                    "inline ![[tx02/page]] after\n"
                    "![[tx02/page|alias]]\n"
                    "\\![[tx02/page]]\n"
                    "! [[tx02/page]]\n"
                    "`![[tx02/page]]`\n"
                    "```md\n"
                    "![[tx02/page]]\n"
                    "```\n"
                    "![[tx02/page]]\n"
                    "![[tx02/other]]\n"
                    "\n"
                    "    ![[tx02/page]]\n",
                ),
                source("tx02/page", "# Page\n"),
                source("tx02/other", "# Other\n"),
            ],
            {
                "tx02/host": document(
                    navigation_links=[
                        nav("tx02/page", None, None, 4),
                        nav("tx02/page", None, None, 5),
                    ],
                    images=[
                        image("tx02/page", "tx02/page"),
                        image("tx02/page", "alias"),
                        image("tx02/page", "tx02/page"),
                        image("tx02/other", "tx02/other"),
                    ],
                    heading_slugs=["exclusions"],
                ),
                "tx02/page": document(heading_slugs=["page"]),
                "tx02/other": document(heading_slugs=["other"]),
            },
            {
                "tx02/host": page(
                    ["exclusions"],
                    ["exclusions"],
                    [{"page": "tx02/page", "anchor": None}, {"page": "tx02/page", "anchor": None}],
                    [],
                    ["tx02/page", "tx02/page", "tx02/page", "tx02/other"],
                    ["alias", "![[tx02/page]]"],
                    ["[transclusion unavailable:", "[transclusion cycle:"],
                )
            },
            [],
            {},
            0,
        ),
        row(
            "TX03",
            [
                source(
                    "tx03/host",
                    "# Contexts\n"
                    "- ![[tx03/tight]]\n"
                    "- adjacent ![[tx03/adjacent]]\n"
                    "\n"
                    "- ![[tx03/loose]]\n"
                    "\n"
                    "> ![[tx03/quote]]\n"
                    ">\n"
                    "> - ![[tx03/nested]]\n",
                ),
                source("tx03/tight", "# Tight\ntight body\n"),
                source("tx03/loose", "# Loose\nloose body\n"),
                source("tx03/quote", "# Quote\nquote body\n"),
                source("tx03/nested", "# Nested\nnested body\n"),
                source("tx03/adjacent", "# Adjacent\nadjacent body\n"),
            ],
            {
                "tx03/host": document(
                    transclusions=[
                        tx(0, "tx03/tight", None, "tx03/tight", 2),
                        tx(1, "tx03/loose", None, "tx03/loose", 5),
                        tx(2, "tx03/quote", None, "tx03/quote", 7),
                        tx(3, "tx03/nested", None, "tx03/nested", 9),
                    ],
                    images=[image("tx03/adjacent", "tx03/adjacent")],
                    heading_slugs=["contexts"],
                ),
                "tx03/tight": document(heading_slugs=["tight"]),
                "tx03/loose": document(heading_slugs=["loose"]),
                "tx03/quote": document(heading_slugs=["quote"]),
                "tx03/nested": document(heading_slugs=["nested"]),
                "tx03/adjacent": document(heading_slugs=["adjacent"]),
            },
            {
                "tx03/host": page(
                    ["contexts"],
                    ["contexts", "tx-0-tight", "tx-1-loose", "tx-2-quote", "tx-3-nested"],
                    [],
                    [],
                    ["tx03/adjacent"],
                    ["tight body", "loose body", "quote body", "nested body"],
                    ["adjacent body"],
                )
            },
            [],
            {},
            0,
        ),
        row(
            "TX04",
            [
                source("tx04/host", "# Host\n![[tx04/sections#keep]]\n"),
                source(
                    "tx04/sections",
                    "# Root\nroot\n## Keep\nkeep\n### Child\nchild\n## Peer\npeer\n# Higher\nhigher\n",
                ),
            ],
            {
                "tx04/host": document(
                    transclusions=[tx(0, "tx04/sections", "keep", "tx04/sections#keep", 2)],
                    heading_slugs=["host"],
                ),
                "tx04/sections": document(heading_slugs=["root", "keep", "child", "peer", "higher"]),
            },
            {
                "tx04/host": page(
                    ["host"],
                    ["host", "tx-0-keep", "tx-0-child"],
                    [],
                    [],
                    [],
                    ["keep", "child"],
                    ["peer", "higher", "root"],
                )
            },
            [],
            {},
            0,
        ),
        row(
            "TX05",
            [
                source("tx05/host", "# Host\n![[tx05/source#target]]\n"),
                source(
                    "tx05/source",
                    "# Source\n"
                    "1. first\n"
                    "2. > ## Target\n"
                    "   > target body\n"
                    "   >\n"
                    "   > ### Child\n"
                    "   > child body\n"
                    "   >\n"
                    "   > ## Peer\n"
                    "   > peer\n"
                    "3. third\n",
                ),
            ],
            {
                "tx05/host": document(
                    transclusions=[tx(0, "tx05/source", "target", "tx05/source#target", 2)],
                    heading_slugs=["host"],
                ),
                "tx05/source": document(heading_slugs=["source", "target", "child", "peer"]),
            },
            {
                "tx05/host": page(
                    ["host"],
                    ["host", "tx-0-target", "tx-0-child"],
                    [],
                    [],
                    [],
                    ["target body", "child body"],
                    ["first", "peer", "third"],
                )
            },
            [],
            {},
            0,
            selector_queries=[
                {
                    "source": "tx05/source",
                    "anchor": "target",
                    "selected_heading_slugs": ["target", "child"],
                    "excluded_heading_slugs": ["source", "peer"],
                    "wrapper": "ordered-list-item-2-block-quote",
                    "ordered_start": 2,
                }
            ],
        ),
        row(
            "TX06",
            [
                source("tx06/a", "# A\n![[tx06/b#keep]]\n"),
                source(
                    "tx06/b",
                    "# B\n## Keep\nbefore\n\n![[tx06/c]]\n\nafter\n## Other\nother\n",
                ),
                source("tx06/c", "# C\nc body\n"),
            ],
            {
                "tx06/a": document(
                    transclusions=[tx(0, "tx06/b", "keep", "tx06/b#keep", 2)],
                    heading_slugs=["a"],
                ),
                "tx06/b": document(
                    transclusions=[tx(0, "tx06/c", None, "tx06/c", 5)],
                    heading_slugs=["b", "keep", "other"],
                ),
                "tx06/c": document(heading_slugs=["c"]),
            },
            {
                "tx06/a": page(
                    ["a"],
                    ["a", "tx-0-keep", "tx-0-0-c"],
                    [],
                    [],
                    [],
                    ["before", "c body", "after"],
                    ["other"],
                )
            },
            [],
            {},
            0,
        ),
        row(
            "TX07",
            [
                source(
                    "tx07/host",
                    "# Controls\n"
                    "![[tx07/exact]]\n"
                    "\n"
                    "![[unique]]\n"
                    "\n"
                    "![[tx07/missing]]\n"
                    "\n"
                    "![[common]]\n"
                    "\n"
                    "![[tx07/exact#nope]]\n",
                ),
                source("tx07/exact", "# Exact\nexact body\n"),
                source("tx07/path/unique", "# Unique\nunique body\n"),
                source("tx07/a/common", "# Common A\ncommon a\n"),
                source("tx07/b/common", "# Common B\ncommon b\n"),
            ],
            {
                "tx07/host": document(
                    transclusions=[
                        tx(0, "tx07/exact", None, "tx07/exact", 2),
                        tx(1, "unique", None, "unique", 4),
                        tx(2, "tx07/missing", None, "tx07/missing", 6),
                        tx(3, "common", None, "common", 8),
                        tx(4, "tx07/exact", "nope", "tx07/exact#nope", 10),
                    ],
                    heading_slugs=["controls"],
                ),
                "tx07/exact": document(heading_slugs=["exact"]),
                "tx07/path/unique": document(heading_slugs=["unique"]),
                "tx07/a/common": document(heading_slugs=["common-a"]),
                "tx07/b/common": document(heading_slugs=["common-b"]),
            },
            {
                "tx07/host": page(
                    ["controls"],
                    ["controls", "tx-0-exact", "tx-1-unique"],
                    [],
                    [],
                    [],
                    [
                        "exact body",
                        "unique body",
                        "[transclusion unavailable: tx07/missing]",
                        "[transclusion unavailable: common]",
                        "[transclusion unavailable: tx07/exact#nope]",
                    ],
                    [],
                )
            },
            [
                {"kind": "broken_transclusion", "from": "tx07/host", "target": "tx07/missing"},
                {
                    "kind": "ambiguous_transclusion",
                    "from": "tx07/host",
                    "target": "common",
                    "candidates": ["tx07/a/common", "tx07/b/common"],
                },
                {
                    "kind": "missing_transclusion_anchor",
                    "from": "tx07/host",
                    "target": "tx07/exact",
                    "heading": "nope",
                },
            ],
            {"broken_transclusion": 1, "ambiguous_transclusion": 1, "missing_transclusion_anchor": 1},
            1,
        ),
        row(
            "TX08",
            [
                source("tx08/a", "# A\n## One\n![[tx08/a#one]]\n## Two\n![[tx08/b#two]]\n"),
                source("tx08/b", "# B\n## Two\n![[tx08/a#two]]\n"),
            ],
            {
                "tx08/a": document(
                    transclusions=[
                        tx(0, "tx08/a", "one", "tx08/a#one", 3),
                        tx(1, "tx08/b", "two", "tx08/b#two", 5),
                    ],
                    heading_slugs=["a", "one", "two"],
                ),
                "tx08/b": document(
                    transclusions=[tx(0, "tx08/a", "two", "tx08/a#two", 3)],
                    heading_slugs=["b", "two"],
                ),
            },
            {
                "tx08/b": page(
                    ["b", "two"],
                    ["b", "two", "tx-0-two"],
                    [],
                    [],
                    [],
                    ["[transclusion cycle: tx08/a#two -> tx08/b#two -> tx08/a#two]"],
                    [],
                )
            },
            [
                {"kind": "transclusion_cycle", "path": ["tx08/a#one", "tx08/a#one"]},
                {"kind": "transclusion_cycle", "path": ["tx08/a#two", "tx08/b#two", "tx08/a#two"]},
            ],
            {"transclusion_cycle": 2},
            1,
        ),
        row(
            "TX09",
            [
                source("tx09/a", "# A\n![[tx09/b#keep]]\n"),
                source("tx09/b", "# B\n## Keep\nkeep\n## Other\n![[tx09/a]]\n"),
                source("tx09/c", "# C\n![[tx09/c]]\n"),
            ],
            {
                "tx09/a": document(
                    transclusions=[tx(0, "tx09/b", "keep", "tx09/b#keep", 2)],
                    heading_slugs=["a"],
                ),
                "tx09/b": document(
                    transclusions=[tx(0, "tx09/a", None, "tx09/a", 5)],
                    heading_slugs=["b", "keep", "other"],
                ),
                "tx09/c": document(
                    transclusions=[tx(0, "tx09/c", None, "tx09/c", 2)],
                    heading_slugs=["c"],
                ),
            },
            {
                "tx09/a": page(
                    ["a"],
                    ["a", "tx-0-keep"],
                    [],
                    [],
                    [],
                    ["keep"],
                    ["other", "[transclusion cycle:"],
                ),
                "tx09/c": page(
                    ["c"],
                    ["c"],
                    [],
                    [],
                    [],
                    ["[transclusion cycle: tx09/c -> tx09/c]"],
                    [],
                ),
            },
            [{"kind": "transclusion_cycle", "path": ["tx09/c", "tx09/c"]}],
            {"transclusion_cycle": 1},
            1,
        ),
        row(
            "TX10",
            [
                source("tx10/a", "# A\n![[tx10/b#one]]\n\n![[tx10/c#one]]\n"),
                source("tx10/b", "# B\n## One\n![[tx10/c#one]]\n"),
                source("tx10/c", "# C\n## One\n![[tx10/a]]\n\n![[tx10/b#one]]\n"),
            ],
            {
                "tx10/a": document(
                    transclusions=[
                        tx(0, "tx10/b", "one", "tx10/b#one", 2),
                        tx(1, "tx10/c", "one", "tx10/c#one", 4),
                    ],
                    heading_slugs=["a"],
                ),
                "tx10/b": document(
                    transclusions=[tx(0, "tx10/c", "one", "tx10/c#one", 3)],
                    heading_slugs=["b", "one"],
                ),
                "tx10/c": document(
                    transclusions=[
                        tx(0, "tx10/a", None, "tx10/a", 3),
                        tx(1, "tx10/b", "one", "tx10/b#one", 5),
                    ],
                    heading_slugs=["c", "one"],
                ),
            },
            {
                "tx10/a": page(
                    ["a"],
                    ["a", "tx-0-one", "tx-0-0-one", "tx-1-one"],
                    [],
                    [],
                    [],
                    [
                        "[transclusion cycle: tx10/a -> tx10/b#one -> tx10/c#one -> tx10/a]",
                        "[transclusion cycle: tx10/b#one -> tx10/c#one -> tx10/b#one]",
                    ],
                    [],
                )
            },
            [
                {
                    "kind": "transclusion_cycle",
                    "path": ["tx10/a", "tx10/b#one", "tx10/c#one", "tx10/a"],
                },
                {"kind": "transclusion_cycle", "path": ["tx10/b#one", "tx10/c#one", "tx10/b#one"]},
            ],
            {"transclusion_cycle": 2},
            1,
        ),
        row(
            "TX11",
            [
                source(
                    "tx11/host",
                    "# Host\n## tx-0-part\n\n![[tx11/source#part]]\n\n![[tx11/source#part]]\n",
                ),
                source(
                    "tx11/source",
                    "# Source\n## Part\nsource body\n[keep](#part)\n[miss](#unknown)\n[[tx11/other]]\n![pic](pic.png)\n",
                ),
                source("tx11/other", "# Other\nother body\n"),
            ],
            {
                "tx11/host": document(
                    transclusions=[
                        tx(0, "tx11/source", "part", "tx11/source#part", 4),
                        tx(1, "tx11/source", "part", "tx11/source#part", 6),
                    ],
                    heading_slugs=["host", "tx-0-part"],
                ),
                "tx11/source": document(
                    navigation_links=[nav("tx11/other", None, None, 6)],
                    images=[image("pic.png", "pic")],
                    heading_slugs=["source", "part"],
                ),
                "tx11/other": document(heading_slugs=["other"]),
            },
            {
                "tx11/host": page(
                    ["host", "tx-0-part"],
                    ["host", "tx-0-part", "tx-0-part-2", "tx-1-part"],
                    [
                        {"page": "tx11/other", "anchor": None},
                        {"page": "tx11/other", "anchor": None},
                    ],
                    ["#tx-0-part-2", "source.md#unknown", "#tx-1-part", "source.md#unknown"],
                    ["pic.png", "pic.png"],
                    ["source body", "keep", "miss"],
                    ["Source", "Other"],
                )
            },
            [],
            {},
            0,
        ),
        row(
            "TX12",
            [
                source("tx12/é:dir/host", "# Host\n![[tx12/é:dir/origin#paths]]\n"),
                source("tx12/outer", "# Outer\n![[tx12/é:dir/origin#paths]]\n"),
                source(
                    "tx12/é:dir/origin",
                    "# Origin\n## Paths\n"
                    "[rel](../guide.md?x=1#frag)\n"
                    "[query](?q=1)\n"
                    "[scheme](./a:b.md)\n"
                    "[web](https://example.com/a.md?x=1#f)\n"
                    "[absolute](/abs/path)\n"
                    "[proto](//cdn.example/x)\n"
                    "[empty]()\n"
                    "![relimg](../img.png)\n"
                    "![schemeimg](./a:b.png)\n",
                ),
            ],
            {
                "tx12/é:dir/host": document(
                    transclusions=[
                        tx(0, "tx12/é:dir/origin", "paths", "tx12/é:dir/origin#paths", 2)
                    ],
                    heading_slugs=["host"],
                ),
                "tx12/outer": document(
                    transclusions=[
                        tx(0, "tx12/é:dir/origin", "paths", "tx12/é:dir/origin#paths", 2)
                    ],
                    heading_slugs=["outer"],
                ),
                "tx12/é:dir/origin": document(
                    images=[image("../img.png", "relimg"), image("./a:b.png", "schemeimg")],
                    heading_slugs=["origin", "paths"],
                ),
            },
            {
                "tx12/é:dir/host": page(
                    ["host"],
                    ["host", "tx-0-paths"],
                    [],
                    [
                        "../guide.md?x=1#frag",
                        "origin.md?q=1",
                        "./a:b.md",
                        "https://example.com/a.md?x=1#f",
                        "/abs/path",
                        "//cdn.example/x",
                        "origin.md",
                    ],
                    ["../img.png", "./a:b.png"],
                    ["rel", "query", "scheme", "web", "absolute", "proto", "empty", "relimg", "schemeimg"],
                    ["[transclusion unavailable:"],
                ),
                "tx12/outer": page(
                    ["outer"],
                    ["outer", "tx-0-paths"],
                    [],
                    [
                        "guide.md?x=1#frag",
                        "%C3%A9%3Adir/origin.md?q=1",
                        "%C3%A9%3Adir/a:b.md",
                        "https://example.com/a.md?x=1#f",
                        "/abs/path",
                        "//cdn.example/x",
                        "%C3%A9%3Adir/origin.md",
                    ],
                    ["img.png", "%C3%A9%3Adir/a:b.png"],
                    ["rel", "query", "scheme", "web", "absolute", "proto", "empty", "relimg", "schemeimg"],
                    ["[transclusion unavailable:"],
                )
            },
            [],
            {},
            0,
            expected_html_contains={
                "../guide.md?x=1#frag": "../guide.html?x=1#frag",
                "origin.md?q=1": "origin.html?q=1",
                "./a:b.md": "./a:b.html",
                "https://example.com/a.md?x=1#f": "https://example.com/a.md?x=1#f",
                "origin.md": "origin.html",
                "guide.md?x=1#frag": "guide.html?x=1#frag",
                "%C3%A9%3Adir/origin.md?q=1": "%C3%A9%3Adir/origin.html?q=1",
                "%C3%A9%3Adir/a:b.md": "%C3%A9%3Adir/a:b.html",
                "%C3%A9%3Adir/origin.md": "%C3%A9%3Adir/origin.html",
            },
        ),
    ]


def json_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def validate_shape(items: list[dict[str, Any]]) -> list[str]:
    errors: list[str] = []
    if [item["id"] for item in items] != [f"TX{i:02d}" for i in range(1, 13)]:
        errors.append("case ids are not TX01..TX12 in order")
    for item in items:
        if item.get("kind") != "site":
            errors.append(f"{item.get('id')}: kind is not site")
        if item["expected_check"]["pages"] != len(item["sources"]):
            errors.append(f"{item['id']}: expected_check.pages mismatch")
        source_paths = [entry["path"] for entry in item["sources"]]
        if set(item["expected_documents"]) != set(source_paths):
            errors.append(f"{item['id']}: expected_documents does not cover every source")
        for path, doc in item["expected_documents"].items():
            for descriptor in doc["transclusions"]:
                if descriptor["id"] < 0:
                    errors.append(f"{item['id']}/{path}: negative transclusion id")
            if any(link["line"] < 1 for link in doc["navigation_links"]):
                errors.append(f"{item['id']}/{path}: invalid navigation line")
        for path, fields in item["expected_pages"].items():
            if path not in item["expected_documents"]:
                errors.append(f"{item['id']}: expected_pages names an unknown page {path}")
            required = {
                "toc_anchors",
                "heading_ids",
                "wiki_targets",
                "ordinary_hrefs",
                "image_sources",
                "text_includes",
                "text_excludes",
            }
            if set(fields) != required:
                errors.append(f"{item['id']}/{path}: expected_pages fields differ")
    return errors


def self_test() -> tuple[bool, str]:
    generated = cases()
    errors = validate_shape(generated)
    case_file = PACKAGE / "CASES.json"
    for relative in PACKAGE_FILES:
        if not (PACKAGE / relative).is_file():
            errors.append(f"package file is missing: {relative}")
    if not case_file.is_file():
        errors.append("CASES.json is missing")
    else:
        actual = case_file.read_bytes()
        expected = json_bytes(generated)
        if actual != expected:
            errors.append("CASES.json differs from deterministic generator output")
        else:
            try:
                loaded = json.loads(actual)
            except json.JSONDecodeError as exc:
                errors.append(f"CASES.json is not JSON: {exc}")
            else:
                if loaded != generated:
                    errors.append("CASES.json JSON value differs from generated value")
    if errors:
        return False, "FAIL\n" + "\n".join(f"- {error}" for error in errors) + "\n"
    return True, (
        "PASS\n"
        "- deterministic generator produced 12 site rows (TX01..TX12)\n"
        "- CASES.json byte comparison matched generator output\n"
        "- every source path is covered by expected_documents\n"
        "- every expected_pages object has the seven declared page fields\n"
        "- no target implementation, renderer, or live oracle was invoked\n"
    )


def reproduce(destination: str) -> int:
    requested = Path(destination)
    if requested.is_absolute() or len(requested.parts) != 1 or requested.parts[0] in {".", ".."}:
        print("refusing reproduction: destination must be one fresh direct child", file=sys.stderr)
        return 2
    out = PACKAGE / requested.parts[0]
    if out.exists() and (not out.is_dir() or any(out.iterdir())):
        print("refusing reproduction: child directory must be fresh and empty", file=sys.stderr)
        return 2
    out.mkdir()
    for relative in PACKAGE_FILES:
        destination_file = out / relative
        destination_file.parent.mkdir(parents=True, exist_ok=True)
        if relative == "CASES.json":
            destination_file.write_bytes(json_bytes(cases()))
        else:
            shutil.copyfile(PACKAGE / relative, destination_file)
    if any((PACKAGE / relative).read_bytes() != (out / relative).read_bytes() for relative in PACKAGE_FILES):
        print("reproduction byte comparison failed", file=sys.stderr)
        return 1
    print(f"reproduced {len(PACKAGE_FILES)} package files under {out.name}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reproduce-to", metavar="SUBDIRECTORY")
    args = parser.parse_args()
    if args.reproduce_to is not None:
        return reproduce(args.reproduce_to)
    ok, report = self_test()
    print(report, end="")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
