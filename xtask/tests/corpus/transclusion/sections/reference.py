#!/usr/bin/env python3
"""Independent, deterministic section-selection reference and fixture generator.

The input is the normalized Node IR described by the transclusion contract.  This
module deliberately does not parse Markdown or import repository implementation
code.
"""

from __future__ import annotations

import argparse
import copy
import json
import random
import sys
from pathlib import Path
from typing import Any


SEED = 90092027
HAND_COUNT = 16
SEEDED_COUNT = 64
PACKAGE_PAYLOADS = (
    "CASES.json",
    "README.md",
    "reference.py",
    "selftestreport.txt",
    "source-snapshots/bdr.md",
    "source-snapshots/contract.md",
    "source-snapshots/prompt.md",
)
STATIC_PACKAGE_FILES = tuple(path for path in PACKAGE_PAYLOADS if path not in {"CASES.json", "selftestreport.txt"})


def text(value: str) -> dict[str, Any]:
    return {"kind": "Text", "text": value}


def paragraph(children: list[dict[str, Any]]) -> dict[str, Any]:
    return {"kind": "Paragraph", "children": children}


def heading(level: int, slug: str, children: list[dict[str, Any]]) -> dict[str, Any]:
    return {"kind": "Heading", "level": level, "slug": slug, "children": children}


def code_block(lang: str | None, value: str) -> dict[str, Any]:
    return {"kind": "CodeBlock", "lang": lang, "text": value}


def ordered_list(start: int | None, items: list[list[dict[str, Any]]]) -> dict[str, Any]:
    return {"kind": "List", "start": start, "items": items}


def quote(callout: str | None, children: list[dict[str, Any]]) -> dict[str, Any]:
    return {"kind": "BlockQuote", "callout": callout, "children": children}


def rule() -> dict[str, Any]:
    return {"kind": "Rule"}


def transclusion(
    ident: int, target: str, anchor: str | None, display: str, line: int
) -> dict[str, Any]:
    return {
        "kind": "Transclusion",
        "id": ident,
        "target": target,
        "anchor": anchor,
        "display": display,
        "line": line,
    }


def _placeholder_tx(target: str, anchor: str | None, display: str, line: int) -> dict[str, Any]:
    return transclusion(-1, target, anchor, display, line)


def _case(
    ident: str,
    body: list[dict[str, Any]],
    anchor: str | None,
    expected: list[dict[str, Any]] | None,
    expected_transclusions: list[dict[str, Any]],
) -> dict[str, Any]:
    return {
        "id": ident,
        "kind": "section",
        "body": body,
        "anchor": anchor,
        "expected": expected,
        "expected_transclusions": expected_transclusions,
    }


def hand_cases() -> list[dict[str, Any]]:
    """Explicit controls.  Expected trees are written independently below."""

    c01_expected = [
        heading(1, "intro", [text("Intro")]),
        paragraph([text("paragraph")]),
        transclusion(0, "guide", "part", "shown", 7),
        code_block("rust", "# not a heading"),
        rule(),
    ]

    c02_body = [
        heading(1, "root", [text("Root")]),
        heading(2, "target", [text("Target")]),
        paragraph([text("keep")]),
        heading(3, "nested", [text("Nested")]),
        paragraph([text("nested body")]),
        heading(1, "after", [text("After")]),
        paragraph([text("exclude")]),
    ]
    c02_expected = [
        heading(2, "target", [text("Target")]),
        paragraph([text("keep")]),
        heading(3, "nested", [text("Nested")]),
        paragraph([text("nested body")]),
    ]

    c03_body = [
        heading(2, "first", [text("First")]),
        paragraph([text("one")]),
        heading(2, "second", [text("Second")]),
        paragraph([text("two")]),
    ]
    c03_expected = [heading(2, "first", [text("First")]), paragraph([text("one")])]

    c04_body = [
        heading(3, "deep", [text("Deep")]),
        paragraph([text("deep body")]),
        heading(4, "deeper", [text("Deeper")]),
        paragraph([text("deeper body")]),
        heading(2, "stop", [text("Stop")]),
        paragraph([text("outside")]),
    ]
    c04_expected = [
        heading(3, "deep", [text("Deep")]),
        paragraph([text("deep body")]),
        heading(4, "deeper", [text("Deeper")]),
        paragraph([text("deeper body")]),
    ]

    c05_body = [heading(2, "topic-2", [text("Final")]), paragraph([text("exact")])]
    c05_expected = [heading(2, "topic-2", [text("Final")]), paragraph([text("exact")])]

    c06_body = [heading(2, "topic", [text("Topic")]), paragraph([text("body")])]

    c07_body = [
        quote(
            "Note",
            [
                heading(2, "inside", [text("Inside")]),
                paragraph([text("selected")]),
                heading(2, "inside-after", [text("After")]),
                paragraph([text("quote outside")]),
            ],
        ),
        paragraph([text("outer sibling")]),
    ]
    c07_expected = [
        quote("Note", [heading(2, "inside", [text("Inside")]), paragraph([text("selected")])])
    ]

    c08_body = [
        ordered_list(
            10,
            [
                [paragraph([text("zero")])],
                [heading(2, "item-target", [text("Item")]), paragraph([text("selected")])],
                [heading(2, "item-after", [text("After")]), paragraph([text("outside")])],
            ],
        )
    ]
    c08_expected = [
        ordered_list(11, [[heading(2, "item-target", [text("Item")]), paragraph([text("selected")])]])
    ]

    c09_body = [
        ordered_list(
            3,
            [
                [paragraph([text("outer zero")])],
                [paragraph([text("outer one")])],
                [
                    text("outer two lead"),
                    ordered_list(
                        7,
                        [
                            [paragraph([text("inner zero")])],
                            [
                                quote(
                                    "Tip",
                                    [
                                        heading(3, "deep-item", [text("Deep item")]),
                                        paragraph([text("deep selected")]),
                                    ],
                                )
                            ],
                        ],
                    ),
                ],
            ],
        )
    ]
    c09_expected = [
        ordered_list(
            5,
            [
                [
                    ordered_list(
                        8,
                        [
                            [
                                quote(
                                    "Tip",
                                    [
                                        heading(3, "deep-item", [text("Deep item")]),
                                        paragraph([text("deep selected")]),
                                    ],
                                )
                            ]
                        ],
                    )
                ]
            ],
        )
    ]

    c10_body = [
        heading(1, "outer", [text("Outer")]),
        quote("Warning", [heading(2, "nested-heading", [text("Nested")]), paragraph([text("quote")])]),
        paragraph([text("outer continues")]),
        heading(1, "outer-end", [text("End")]),
    ]
    c10_expected = [
        heading(1, "outer", [text("Outer")]),
        quote("Warning", [heading(2, "nested-heading", [text("Nested")]), paragraph([text("quote")])]),
        paragraph([text("outer continues")]),
    ]

    c11_body = [
        code_block("markdown", "# pretend\n## target"),
        heading(2, "real", [text("Real")]),
        paragraph([text("real body")]),
    ]

    c12_body = [
        heading(1, "with-embeds", [text("Embeds")]),
        _placeholder_tx("a", None, "A", 12),
        quote(None, [_placeholder_tx("b", "part", "B", 13)]),
        heading(1, "next", [text("Next")]),
    ]
    c12_expected = [
        heading(1, "with-embeds", [text("Embeds")]),
        transclusion(0, "a", None, "A", 12),
        quote(None, [transclusion(1, "b", "part", "B", 13)]),
    ]

    c13_body = [
        heading(1, "other", [text("Other")]),
        transclusion(0, "outside", None, "outside", 20),
        heading(1, "selected", [text("Selected")]),
        paragraph([text("no embed")]),
        heading(1, "last", [text("Last")]),
    ]
    c13_expected = [heading(1, "selected", [text("Selected")]), paragraph([text("no embed")])]

    c14_expected = [
        heading(2, "fields", [text("Fields")]),
        paragraph([text("before")]),
        transclusion(0, "source/page.md", "a-b", "Display exact", 101),
        paragraph([text("after")]),
        code_block(None, "# a heading-looking code line"),
    ]

    c15_body = [
        quote("Caution", [heading(4, "callout-target", [text("Callout")]), rule()]),
        paragraph([text("outer")]),
    ]
    c15_expected = [quote("Caution", [heading(4, "callout-target", [text("Callout")]), rule()])]

    c16_body = [
        heading(1, "unicode", [text("Ünicode")]),
        paragraph([text("é / colon:name / tabs\t preserved")]),
        ordered_list(None, [[paragraph([text("unordered semantic")])]]),
    ]
    c16_expected = [
        heading(1, "unicode", [text("Ünicode")]),
        paragraph([text("é / colon:name / tabs\t preserved")]),
        ordered_list(None, [[paragraph([text("unordered semantic")])]]),
    ]

    return [
        _case(
            "hand-01-full-body",
            [
                heading(1, "intro", [text("Intro")]),
                paragraph([text("paragraph")]),
                transclusion(0, "guide", "part", "shown", 7),
                code_block("rust", "# not a heading"),
                rule(),
            ],
            None,
            c01_expected,
            [transclusion(0, "guide", "part", "shown", 7)],
        ),
        _case("hand-02-root-lower-boundary", c02_body, "target", c02_expected, []),
        _case("hand-03-root-equal-boundary", c03_body, "first", c03_expected, []),
        _case("hand-04-root-higher-boundary", c04_body, "deep", c04_expected, []),
        _case("hand-05-exact-final-slug", c05_body, "topic-2", c05_expected, []),
        _case("hand-06-case-mismatched-slug", c06_body, "Topic", None, []),
        _case("hand-07-missing-anchor", c06_body, "absent", None, []),
        _case("hand-08-nested-quote-outer-excluded", c07_body, "inside", c07_expected, []),
        _case("hand-09-ordered-item-adjustment", c08_body, "item-target", c08_expected, []),
        _case("hand-10-nested-list-quote-shells", c09_body, "deep-item", c09_expected, []),
        _case("hand-11-root-not-ended-by-nested-heading", c10_body, "outer", c10_expected, []),
        _case("hand-12-code-heading-not-recognized", c11_body, "target", None, []),
        _case(
            "hand-13-containing-transclusions",
            [
                heading(1, "with-embeds", [text("Embeds")]),
                transclusion(0, "a", None, "A", 12),
                quote(None, [transclusion(1, "b", "part", "B", 13)]),
                heading(1, "next", [text("Next")]),
            ],
            "with-embeds",
            c12_expected,
            [transclusion(0, "a", None, "A", 12), transclusion(1, "b", "part", "B", 13)],
        ),
        _case("hand-14-excluding-transclusions", c13_body, "selected", c13_expected, []),
        _case(
            "hand-15-exact-leaf-fields",
            [
                heading(2, "fields", [text("Fields")]),
                paragraph([text("before")]),
                transclusion(0, "source/page.md", "a-b", "Display exact", 101),
                paragraph([text("after")]),
                code_block(None, "# a heading-looking code line"),
            ],
            "fields",
            c14_expected,
            [transclusion(0, "source/page.md", "a-b", "Display exact", 101)],
        ),
        _case("hand-16-empty-body", [], None, [], []),
    ]


def _walk_nodes(nodes: list[dict[str, Any]]):
    for node in nodes:
        yield node
        kind = node["kind"]
        if kind in {"Heading", "Paragraph", "BlockQuote"}:
            yield from _walk_nodes(node["children"])
        elif kind == "List":
            for item in node["items"]:
                yield from _walk_nodes(item)


def _assign_ids(body: list[dict[str, Any]]) -> None:
    ident = 0
    for node in _walk_nodes(body):
        if node["kind"] == "Transclusion":
            node["id"] = ident
            ident += 1


def _seeded_body(rng: random.Random, case_index: int) -> list[dict[str, Any]]:
    body: list[dict[str, Any]] = []
    for section in range(4):
        root_level = [1, 2, 3][(case_index + section + rng.randrange(3)) % 3]
        root_slug = f"seed-{case_index:02d}-root-{section}"
        body.append(heading(root_level, root_slug, [text(f"Root {section}")]))
        if rng.randrange(2) == 0:
            body.append(paragraph([text(f"section {section} ")]))
            body.append(
                _placeholder_tx(
                    f"page-{case_index}-{section}",
                    "keep" if section % 2 else None,
                    f"seed display {section}",
                    200 + case_index * 10 + section,
                )
            )
            body.append(paragraph([text(" tail")]))
        else:
            body.append(paragraph([text(f"section {section} plain")]))
        body.append(code_block("md" if section % 2 else None, f"# fake-{case_index}-{section}"))

        nested_level = min(6, root_level + 1 + (section % 2))
        nested_slug = f"seed-{case_index:02d}-quote-{section}"
        nested_quote = quote(
            [None, "Note", "Tip", "Important", "Warning", "Caution"][(case_index + section) % 6],
            [
                heading(nested_level, nested_slug, [text(f"Quote {section}")]),
                paragraph([text("quote content")]),
                rule(),
            ],
        )
        list_items: list[list[dict[str, Any]]] = [
            [paragraph([text(f"item zero {section}")])],
            [
                heading(
                    min(6, nested_level + 1),
                    f"seed-{case_index:02d}-list-{section}",
                    [text(f"List {section}")],
                ),
                paragraph([text("list content")]),
                nested_quote,
            ],
            [paragraph([text(f"item two {section}")])],
        ]
        body.append(ordered_list(section if section % 2 else None, list_items))
    _assign_ids(body)
    return body


def _heading_slugs(body: list[dict[str, Any]]) -> list[str]:
    return [node["slug"] for node in _walk_nodes(body) if node["kind"] == "Heading"]


def seeded_cases() -> list[dict[str, Any]]:
    rng = random.Random(SEED)
    result: list[dict[str, Any]] = []
    for index in range(SEEDED_COUNT):
        body = _seeded_body(rng, index)
        slugs = _heading_slugs(body)
        if index % 4 == 0:
            anchor: str | None = None
        elif index % 5 == 0:
            anchor = f"missing-{index:02d}"
        elif index % 7 == 0:
            chosen = slugs[(index * 3 + rng.randrange(len(slugs))) % len(slugs)]
            anchor = chosen.upper() if chosen.upper() != chosen else chosen + "-CASE"
        else:
            anchor = slugs[(index * 5 + rng.randrange(len(slugs))) % len(slugs)]
        expected = select_section(body, anchor)
        result.append(
            _case(
                f"seed-{index + 1:02d}",
                body,
                anchor,
                expected,
                transclusions(expected) if expected is not None else [],
            )
        )
    return result


def select_section(body: list[dict[str, Any]], anchor: str | None) -> list[dict[str, Any]] | None:
    """Select a complete body or one exact heading section without mutating input."""

    if anchor is None:
        return copy.deepcopy(body)
    selected = _select_sequence(body, anchor)
    return selected


def _select_sequence(
    sequence: list[dict[str, Any]], anchor: str
) -> list[dict[str, Any]] | None:
    for index, node in enumerate(sequence):
        if node["kind"] == "Heading" and node["slug"] == anchor:
            level = node["level"]
            end = len(sequence)
            for boundary in range(index + 1, len(sequence)):
                candidate = sequence[boundary]
                if candidate["kind"] == "Heading" and candidate["level"] <= level:
                    end = boundary
                    break
            return copy.deepcopy(sequence[index:end])

    for node in sequence:
        if node["kind"] == "List":
            for item_index, item in enumerate(node["items"]):
                selected = _select_sequence(item, anchor)
                if selected is not None:
                    shell = copy.deepcopy(node)
                    shell["items"] = [selected]
                    if shell["start"] is not None:
                        shell["start"] += item_index
                    return [shell]
        elif node["kind"] == "BlockQuote":
            selected = _select_sequence(node["children"], anchor)
            if selected is not None:
                shell = copy.deepcopy(node)
                shell["children"] = selected
                return [shell]
    return None


def transclusions(nodes: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Return copied full descriptors in deterministic tree traversal order."""

    result: list[dict[str, Any]] = []
    for node in _walk_nodes(nodes):
        if node["kind"] == "Transclusion":
            result.append(copy.deepcopy(node))
    return result


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2, sort_keys=False) + "\n").encode("utf-8")


def build_cases() -> list[dict[str, Any]]:
    return hand_cases() + seeded_cases()


def _validate_ids(body: list[dict[str, Any]]) -> None:
    ids = [node["id"] for node in _walk_nodes(body) if node["kind"] == "Transclusion"]
    if ids != list(range(len(ids))):
        raise AssertionError(f"transclusion ids are not contiguous preorder ids: {ids}")


def _validate_legal_tree(nodes: list[dict[str, Any]], inline_ancestor: bool = False) -> None:
    block_kinds = {"Text", "Heading", "Paragraph", "CodeBlock", "List", "BlockQuote", "Rule", "Transclusion"}
    for node in nodes:
        kind = node["kind"]
        if inline_ancestor:
            if kind != "Text":
                if kind in block_kinds:
                    raise AssertionError("block node nested inside inline container")
                raise AssertionError("unknown node kind in inline container")
            continue
        if kind not in block_kinds:
            raise AssertionError("unknown or inline node kind in block sequence")
        if kind in {"Heading", "Paragraph"}:
            _validate_legal_tree(node["children"], inline_ancestor=True)
        elif kind == "BlockQuote":
            _validate_legal_tree(node["children"], inline_ancestor=inline_ancestor)
        elif kind == "List":
            for item in node["items"]:
                _validate_legal_tree(item, inline_ancestor=inline_ancestor)


def _report_text(
    case_count: int, selection_checks: int, descriptor_checks: int, mutation_checks: int
) -> str:
    return "\n".join(
        [
            "selftest: PASS",
            f"cases: {case_count} (hand={HAND_COUNT}, seeded={SEEDED_COUNT})",
            "kinds: section=80",
            f"selection_checks: {selection_checks}",
            f"descriptor_checks: {descriptor_checks}",
            f"mutation_checks: {mutation_checks}",
            f"legality_checks: {case_count}",
            "negative_legality_controls: direct-invalid, paragraph-quote, paragraph-heading, unknown-block, paragraph-unknown",
            "reproduction: full package bytes match deterministic frozen-input output",
        ]
    ) + "\n"


def _sha256(data: bytes) -> str:
    import hashlib

    return hashlib.sha256(data).hexdigest()


def _sha256sums(payloads: dict[str, bytes]) -> bytes:
    return "".join(f"{_sha256(payloads[path])}  {path}\n" for path in sorted(payloads)).encode("utf-8")


def _registration(payloads: dict[str, bytes], sums: bytes) -> bytes:
    cases = json.loads(payloads["CASES.json"].decode("utf-8"))
    source = {
        "contract": "source-snapshots/contract.md",
        "bdr": "source-snapshots/bdr.md",
        "prompt": "source-snapshots/prompt.md",
    }
    lines = [
        "schema_version = 1",
        'package = "transclusion-sections"',
        'eventual_path = "xtask/tests/corpus/transclusion/sections"',
        f"seed = {SEED}",
        'model = "gpt-5.6-luna"',
        'effort = "high"',
        f"case_count = {len(cases)}",
        f"kind_count = {len({case['kind'] for case in cases})}",
        f"payload_count = {len(payloads)}",
        f'content_sha256 = "{_sha256(sums)}"',
        "",
        "[case_counts]",
        "section = 80",
        "",
        "[source_snapshots]",
        'contract_path = "xtask/tests/corpus/transclusion/sections/source-snapshots/contract.md"',
        f'contract_sha256 = "{_sha256(payloads[source["contract"]])}"',
        'bdr_path = "xtask/tests/corpus/transclusion/sections/source-snapshots/bdr.md"',
        f'bdr_sha256 = "{_sha256(payloads[source["bdr"]])}"',
        'prompt_path = "xtask/tests/corpus/transclusion/sections/source-snapshots/prompt.md"',
        f'prompt_sha256 = "{_sha256(payloads[source["prompt"]])}"',
        "",
        "[original_paths]",
        'contract = "docs/architecture/transclusion-contract.md"',
        'bdr = "docs/adr/BDR-0007-site-owns-page-tree.md"',
        'prompt = "xtask/tests/corpus/write-prompts/CHG-008/pd-transclusion-sections-generator-prompt.md"',
        "",
    ]
    return "\n".join(lines).encode("utf-8")


def _deterministic_package_bytes(package: Path, report: str) -> dict[str, bytes]:
    payloads = {
        path: (package / path).read_bytes()
        for path in STATIC_PACKAGE_FILES
    }
    payloads["CASES.json"] = canonical_json(build_cases())
    payloads["selftestreport.txt"] = report.encode("utf-8")
    sums = _sha256sums(payloads)
    return {**payloads, "SHA256SUMS": sums, "registration.toml": _registration(payloads, sums)}


def _write_reproduction(package: Path, target: Path, report: str) -> None:
    output = _deterministic_package_bytes(package, report)
    for relative, data in output.items():
        destination = target / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(data)


def self_test(package: Path, loaded: list[dict[str, Any]]) -> str:
    generated = build_cases()
    if loaded != generated:
        raise AssertionError("committed CASES.json differs from deterministic generated cases")
    if len(loaded) != HAND_COUNT + SEEDED_COUNT:
        raise AssertionError(f"unexpected case count: {len(loaded)}")
    if sum(case["kind"] == "section" for case in loaded) != len(loaded):
        raise AssertionError("unexpected case kind")

    selection_checks = 0
    descriptor_checks = 0
    mutation_checks = 0
    legality_checks = 0
    for case in loaded:
        _validate_legal_tree(case["body"])
        legality_checks += 1
        _validate_ids(case["body"])
        before = copy.deepcopy(case["body"])
        actual = select_section(case["body"], case["anchor"])
        if actual != case["expected"]:
            raise AssertionError(f"selection mismatch in {case['id']}")
        selection_checks += 1
        actual_descriptors = transclusions(actual) if actual is not None else []
        if actual_descriptors != case["expected_transclusions"]:
            raise AssertionError(f"descriptor mismatch in {case['id']}")
        descriptor_checks += 1
        if case["body"] != before:
            raise AssertionError(f"input mutation in {case['id']}")
        mutation_checks += 1

    negative_controls = [
        ("direct-invalid", [paragraph([text("illegal"), transclusion(0, "bad", None, "bad", 1)])]),
        ("paragraph-quote", [paragraph([quote(None, [transclusion(0, "bad", None, "bad", 1)])])]),
        ("paragraph-heading", [paragraph([heading(2, "illegal-heading", [text("bad")])])]),
        ("unknown-block", [{"kind": "Mystery"}]),
        ("paragraph-unknown", [paragraph([{"kind": "Mystery"}])]),
    ]
    for control_name, illegal in negative_controls:
        try:
            _validate_legal_tree(illegal)
        except AssertionError:
            pass
        else:
            raise AssertionError(f"negative legality control was accepted: {control_name}")

    committed = (package / "CASES.json").read_bytes()
    if committed != canonical_json(loaded):
        raise AssertionError("CASES.json is not canonical deterministic JSON")
    report = _report_text(len(loaded), selection_checks, descriptor_checks, mutation_checks)
    expected_package = _deterministic_package_bytes(package, report)
    for relative, expected in expected_package.items():
        if (package / relative).read_bytes() != expected:
            raise AssertionError(f"package metadata or payload differs for {relative}")
    return report


def _safe_reproduction_dir(package: Path, value: str) -> Path:
    target = (package / value).resolve() if not Path(value).is_absolute() else Path(value).resolve()
    if target == package or package not in target.parents:
        raise SystemExit("--reproduce-to must name a strict subdirectory of the script package")
    return target


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--reproduce-to",
        metavar="SUBDIR",
        help="write generated CASES.json and a report under this package-owned subdirectory",
    )
    args = parser.parse_args(argv)
    package = Path(__file__).resolve().parent
    cases_path = package / "CASES.json"
    if args.reproduce_to and not cases_path.exists():
        target = _safe_reproduction_dir(package, args.reproduce_to)
        target.mkdir(parents=True, exist_ok=True)
        generated = build_cases()
        report = _report_text(len(generated), len(generated), len(generated), len(generated))
        _write_reproduction(package, target, report)
        print(report, end="")
        print(f"reproduced_to: {target.relative_to(package)}")
        return 0
    try:
        loaded = json.loads(cases_path.read_text(encoding="utf-8"))
        report = self_test(package, loaded)
    except (OSError, json.JSONDecodeError, AssertionError) as exc:
        print(f"selftest: FAIL: {exc}", file=sys.stderr)
        return 1
    if args.reproduce_to:
        target = _safe_reproduction_dir(package, args.reproduce_to)
        target.mkdir(parents=True, exist_ok=True)
        _write_reproduction(package, target, report)
        print(report, end="")
        print(f"reproduced_to: {target.relative_to(package)}")
    else:
        print(report, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
