#!/usr/bin/env python3
"""Independent, stdlib-only registration generator for the markdown shape supplement."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
OLD_SOURCE = ROOT / "evidence/md-corpus/20260922T210717Z-58e45f444c9f.json"
NEW_SOURCE = ROOT / "evidence/md-corpus/20260923T044415Z-ed79a7a8e0ac.json"
PROMPT_SOURCE = ROOT / "target/m2/markdown-schema-supplement-prompt.md"
AMENDMENT_SOURCE = ROOT / "docs/architecture/markdown-record-shape-amendment.md"
BASE_SOURCE = ROOT / "docs/architecture/record-schema-contract.md"

SOURCE_PATHS = (
    AMENDMENT_SOURCE,
    BASE_SOURCE,
    OLD_SOURCE,
    NEW_SOURCE,
    PROMPT_SOURCE,
)


def rel(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def json_type(value: Any) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "boolean"
    if isinstance(value, int):
        return "integer"
    if isinstance(value, float):
        return "number"
    if isinstance(value, str):
        return "string"
    if isinstance(value, list):
        return "array"
    if isinstance(value, dict):
        return "object"
    raise TypeError(f"unsupported JSON value: {type(value)!r}")


def load_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def write_json(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def census(document: dict[str, Any]) -> dict[str, Any]:
    cases = document["cases"]
    case_key_sets = [set(case) for case in cases]
    case_union = set().union(*case_key_sets)
    case_intersection = set.intersection(*case_key_sets)
    product = document["product"]
    return {
        "case_count": len(cases),
        "case_key_intersection": sorted(case_intersection),
        "case_key_union": sorted(case_union),
        "case_sites": [case["site"] for case in cases],
        "product_keys": sorted(product),
        "product_types": {
            key: json_type(value) for key, value in sorted(product.items())
        },
        "top_level_keys": sorted(document),
    }


def source_records() -> list[dict[str, str]]:
    return [{"path": rel(path), "sha256": sha256(path)} for path in SOURCE_PATHS]


def pointer_parts(pointer: str) -> list[str]:
    if not pointer.startswith("/"):
        raise ValueError(f"not a JSON pointer: {pointer}")
    return [part.replace("~1", "/").replace("~0", "~") for part in pointer[1:].split("/")]


def parent_and_part(document: Any, pointer: str) -> tuple[Any, str]:
    parts = pointer_parts(pointer)
    if not parts:
        raise ValueError("root mutation is not supported")
    parent = document
    for part in parts[:-1]:
        parent = parent[int(part)] if isinstance(parent, list) else parent[part]
    return parent, parts[-1]


def apply_mutations(document: dict[str, Any], mutations: list[dict[str, Any]]) -> dict[str, Any]:
    result = copy.deepcopy(document)
    for mutation in mutations:
        parent, part = parent_and_part(result, mutation["path"])
        if mutation["op"] == "remove":
            if isinstance(parent, list):
                parent.pop(int(part))
            else:
                del parent[part]
        elif mutation["op"] == "set":
            if isinstance(parent, list):
                parent[int(part)] = copy.deepcopy(mutation["value"])
            else:
                parent[part] = copy.deepcopy(mutation["value"])
        else:
            raise ValueError(f"unsupported mutation operation: {mutation['op']}")
    return result


def literal_case_path(index: int) -> str:
    return f"/cases/{index}/well_formed"


def recipes() -> list[dict[str, Any]]:
    remove_all = [
        {"op": "remove", "path": literal_case_path(index)}
        for index in range(60)
    ]
    remove_all.append({"op": "remove", "path": "/product/binary_sha256"})

    result: list[dict[str, Any]] = [
        {
            "id": "control_old_unchanged",
            "baseline": "old.json",
            "mutations": [],
            "expected_schema_codes": [],
        },
        {
            "id": "control_new_unchanged",
            "baseline": "new.json",
            "mutations": [],
            "expected_schema_codes": [],
        },
        {
            "id": "historical_remove_both_additions",
            "baseline": "new.json",
            "mutations": remove_all,
            "expected_schema_codes": [],
        },
        {
            "id": "well_formed_false_accepted",
            "baseline": "new.json",
            "mutations": [
                {"op": "set", "path": literal_case_path(0), "value": False}
            ],
            "expected_schema_codes": [],
        },
    ]

    for name, value in (
        ("null", None),
        ("integer", 7),
        ("string", "not-a-boolean"),
        ("object", {"value": 7}),
        ("array", [7]),
    ):
        result.append(
            {
                "id": f"well_formed_{name}_wrong_type",
                "baseline": "new.json",
                "mutations": [
                    {"op": "set", "path": literal_case_path(0), "value": value}
                ],
                "expected_schema_codes": ["schema.wrong_type"],
            }
        )

    for name, value in (
        ("null", None),
        ("boolean", False),
        ("integer", 7),
        ("array", [7]),
        ("object", {"value": 7}),
    ):
        result.append(
            {
                "id": f"binary_sha256_{name}_wrong_type",
                "baseline": "new.json",
                "mutations": [
                    {"op": "set", "path": "/product/binary_sha256", "value": value}
                ],
                "expected_schema_codes": ["schema.wrong_type"],
            }
        )

    result.extend(
        [
            {
                "id": "unknown_case_neighbor_rejected",
                "baseline": "new.json",
                "mutations": [
                    {
                        "op": "set",
                        "path": "/cases/0/unknown_neighbor",
                        "value": "rejected",
                    }
                ],
                "expected_schema_codes": ["schema.unknown_field"],
            },
            {
                "id": "unknown_product_neighbor_rejected",
                "baseline": "new.json",
                "mutations": [
                    {
                        "op": "set",
                        "path": "/product/unknown_neighbor",
                        "value": "rejected",
                    }
                ],
                "expected_schema_codes": ["schema.unknown_field"],
            },
        ]
    )
    return result


def structural_codes(document: dict[str, Any], old: dict[str, Any], new: dict[str, Any]) -> list[str]:
    allowed_case_keys = set(old["cases"][0]) | set(new["cases"][0])
    allowed_product_keys = set(old["product"]) | set(new["product"])
    codes: set[str] = set()
    for case in document["cases"]:
        if set(case) - allowed_case_keys:
            codes.add("schema.unknown_field")
        if "well_formed" in case and json_type(case["well_formed"]) != "boolean":
            codes.add("schema.wrong_type")
    product = document["product"]
    if set(product) - allowed_product_keys:
        codes.add("schema.unknown_field")
    if "binary_sha256" in product and json_type(product["binary_sha256"]) != "string":
        codes.add("schema.wrong_type")
    return sorted(codes)


def make_inventory(
    old: dict[str, Any], new: dict[str, Any], sources: list[dict[str, str]], recipes_data: list[dict[str, Any]]
) -> dict[str, Any]:
    old_census = census(old)
    new_census = census(new)
    return {
        "family": "markdown_corpus",
        "sources": sources,
        "census": {"old": old_census, "new": new_census},
        "observed_additions": [
            {
                "instance_path": "/cases/*/well_formed",
                "observed_types": sorted(
                    {json_type(case["well_formed"]) for case in new["cases"]}
                ),
                "old_present": any("well_formed" in case for case in old["cases"]),
                "new_present": all("well_formed" in case for case in new["cases"]),
            },
            {
                "instance_path": "/product/binary_sha256",
                "observed_types": [json_type(new["product"]["binary_sha256"])],
                "old_present": "binary_sha256" in old["product"],
                "new_present": "binary_sha256" in new["product"],
            },
        ],
        "contract": {
            "after_data": True,
            "new_fields_optional": True,
            "objects_closed": True,
            "production_validator_used": False,
            "corpus_grading_changed": False,
        },
        "recipe_count": len(recipes_data),
    }


def self_check(old: dict[str, Any], new: dict[str, Any], recipes_data: list[dict[str, Any]]) -> None:
    old_census = census(old)
    new_census = census(new)
    assert old_census["case_count"] == new_census["case_count"] == 60
    assert old_census["case_sites"] == [f"MD{index:03d}" for index in range(1, 61)]
    assert old_census["case_sites"] == new_census["case_sites"]
    assert "well_formed" not in old_census["case_key_union"]
    assert new_census["product_types"]["binary_sha256"] == "string"
    assert new_census["case_key_intersection"][-1] == "well_formed"
    assert {json_type(case["well_formed"]) for case in new["cases"]} == {"boolean"}
    assert not (set(new_census["top_level_keys"]) - set(old_census["top_level_keys"]))
    assert set(new_census["case_key_union"]) - set(old_census["case_key_union"]) == {"well_formed"}
    assert set(new_census["product_keys"]) - set(old_census["product_keys"]) == {"binary_sha256"}

    by_name = {"old.json": old, "new.json": new}
    for recipe in recipes_data:
        mutated = apply_mutations(by_name[recipe["baseline"]], recipe["mutations"])
        actual = structural_codes(mutated, old, new)
        assert actual == recipe["expected_schema_codes"], (recipe["id"], actual)


def write_sha256sums() -> str:
    rows: list[str] = []
    for path in sorted(OUT.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(OUT).as_posix()
        if relative in {"SHA256SUMS", "registration.toml"}:
            continue
        rows.append(f"{sha256(path)}  {relative}")
    text = "\n".join(rows) + "\n"
    (OUT / "SHA256SUMS").write_text(text, encoding="utf-8")
    return sha256(OUT / "SHA256SUMS")


def write_registration(
    sources: list[dict[str, str]], old: dict[str, Any], new: dict[str, Any], recipes_data: list[dict[str, Any]], content_sha: str
) -> None:
    old_census = census(old)
    new_census = census(new)
    lines = [
        'family = "markdown_corpus"',
        "after_data = true",
        'grading_authority_ledger_id = "CHG-019.1-markdown-shape-amendment"',
        'model_profile = "gpt-5.6-luna high"',
        'prompt_path = "target/m2/markdown-schema-supplement-prompt.md"',
        f'prompt_sha256 = "{next(item["sha256"] for item in sources if item["path"] == rel(PROMPT_SOURCE))}"',
        'content_sha256 = "' + content_sha + '"',
        "",
        "[census]",
        f"old_case_count = {old_census['case_count']}",
        f"new_case_count = {new_census['case_count']}",
        f"old_case_site_count = {len(old_census['case_sites'])}",
        f"new_case_site_count = {len(new_census['case_sites'])}",
        f"source_count = {len(sources)}",
        "",
        "[recipes]",
        f"count = {len(recipes_data)}",
        'expected_code_vocabulary = ["schema.unknown_field", "schema.wrong_type"]',
        "",
        "[content]",
        'sha256sums_path = "SHA256SUMS"',
        'registration_excluded_from_sha256sums = true',
        'sha256sums_excluded_from_sha256sums = true',
    ]
    for source in sources:
        lines.extend(
            [
                "",
                "[[sources]]",
                f'path = "{source["path"]}"',
                f'sha256 = "{source["sha256"]}"',
            ]
        )
    (OUT / "registration.toml").write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    old = load_json(OLD_SOURCE)
    new = load_json(NEW_SOURCE)
    sources = source_records()
    recipes_data = recipes()
    self_check(old, new, recipes_data)

    (OUT / "staging").mkdir(exist_ok=True)
    (OUT / "old.json").write_bytes(OLD_SOURCE.read_bytes())
    (OUT / "new.json").write_bytes(NEW_SOURCE.read_bytes())
    (OUT / "staging/old.json").write_bytes(OLD_SOURCE.read_bytes())
    (OUT / "staging/new.json").write_bytes(NEW_SOURCE.read_bytes())
    additions = {
        "family": "markdown_corpus",
        "optional": [
            {"instance_path": "/cases/*/well_formed", "types": ["boolean"]},
            {"instance_path": "/product/binary_sha256", "types": ["string"]},
        ],
        "source_paths": sources,
        "after_data": True,
    }
    inventory = make_inventory(old, new, sources, recipes_data)
    write_json(OUT / "ADDITIONS.json", additions)
    write_json(OUT / "CASES.json", recipes_data)
    write_json(OUT / "inventory.json", inventory)
    write_json(OUT / "staging/ADDITIONS.json", additions)
    write_json(OUT / "staging/CASES.json", recipes_data)
    write_json(OUT / "staging/inventory.json", inventory)
    content_sha = write_sha256sums()
    write_registration(sources, old, new, recipes_data, content_sha)


if __name__ == "__main__":
    main()
