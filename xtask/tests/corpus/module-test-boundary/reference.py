#!/usr/bin/env python3
"""Bounded, read-only reference for the module-test-source boundary fixture."""

from __future__ import annotations

import hashlib
import json
import os
import posixpath
import re
import shutil
import sys
import tomllib
from pathlib import Path


PACKAGE = Path(__file__).resolve().parent
PAYLOADS = (
    "CASES.json",
    "reference.py",
    "README.md",
    "selftestreport.txt",
    "contractaddendum.md",
    "originalcontract.md",
    "prompt.md",
)
ALL_FILES = PAYLOADS + ("registration.toml", "SHA256SUMS")
ROW_KEYS = {
    "id",
    "kind",
    "mode",
    "crate_name",
    "crate_root",
    "workspace_root",
    "root_file",
    "files",
    "symlinks",
    "expected",
    "mutation",
}
EXPECTED_KEYS = {"outcome", "reason_contains", "modules", "test_edge", "source_files"}
MUTATION_KEYS = {"path", "text", "expected_sha256"}
MODULE_LINE = re.compile(r"^mod\s+([A-Za-z_]\w*)\s*;$")
PATH_ATTR = re.compile(r'^#\[path\s*=\s*"([^\"]+)"\]$')
SHA256 = re.compile(r"^[0-9a-f]{64}$")
SNAPSHOT_SPEC = (
    (
        "contractaddendum.md",
        "docs/architecture/module-test-source-boundary.md",
    ),
    (
        "originalcontract.md",
        "docs/architecture/module-check-contract.md",
    ),
    (
        "prompt.md",
        "xtask/tests/corpus/write-prompts/CHG-008/pd-module-test-boundary-generator-prompt.md",
    ),
)
INVALID_REFUSAL_IDS = {
    "workspace_production_external_helper",
    "workspace_cfg_test_target_outside_workspace",
    "workspace_cfg_test_canonical_symlink_outside",
    "workspace_production_symlink_inside_workspace",
    "workspace_external_inner_cfg_test_non_test_declaration",
    "workspace_external_nonexact_cfg_test_declaration",
    "workspace_external_cfg_attr_path_declaration",
}


class FixtureError(Exception):
    pass


def fail(message: str) -> None:
    raise FixtureError(message)


def normalized(path: str) -> str:
    if not isinstance(path, str) or not path or path.startswith("/"):
        fail(f"unsafe relative path: {path!r}")
    value = posixpath.normpath(path)
    if value in (".", "..") or value.startswith("../"):
        fail(f"path escapes case root: {path!r}")
    return value


def within(path: str, root: str) -> bool:
    return path == root or path.startswith(root + "/")


def canonical(path: str, symlinks: dict[str, str]) -> str:
    current = normalized(path)
    seen: set[str] = set()
    while current in symlinks:
        if current in seen:
            fail(f"symlink cycle at {current}")
        seen.add(current)
        target = symlinks[current]
        if not isinstance(target, str) or target.startswith("/"):
            fail(f"unsafe symlink target at {current}")
        current = normalized(posixpath.join(posixpath.dirname(current), target))
    return current


def digest_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def digest(text: str) -> str:
    return digest_bytes(text.encode("utf-8"))


def boundary_allows(
    path: str,
    crate_root: str,
    workspace_root: str | None,
    inherited_test: bool,
    mode: str,
) -> bool:
    if within(path, crate_root):
        return True
    return (
        inherited_test
        and mode == "workspace"
        and workspace_root is not None
        and within(path, workspace_root)
    )


def parse_module_declarations(source: str) -> list[dict[str, object]]:
    """Parse only conventional `mod name;` lines and their line attributes."""
    lines = source.splitlines()
    declarations: list[dict[str, object]] = []
    for index, raw_line in enumerate(lines):
        line = raw_line.strip()
        match = MODULE_LINE.fullmatch(line)
        if match is None:
            continue
        attributes: list[str] = []
        previous = index - 1
        while previous >= 0:
            candidate = lines[previous].strip()
            if not candidate:
                break
            if not candidate.startswith("#[") or not candidate.endswith("]"):
                break
            attributes.insert(0, candidate)
            previous -= 1
        path: str | None = None
        path_index = -1
        for attr_index, attribute in enumerate(attributes):
            path_match = PATH_ATTR.fullmatch(attribute)
            if path_match is not None:
                if path is not None:
                    fail("bounded fixture language permits one path attribute per module")
                path = path_match.group(1)
                path_index = attr_index
        exact_test = (
            attributes == ["#[cfg(test)]"]
            or (
                path_index == 1
                and attributes[0] == "#[cfg(test)]"
                and len(attributes) == 2
            )
        )
        declarations.append(
            {
                "name": match.group(1),
                "path": path,
                "exact_test": exact_test,
                "attributes": attributes,
            }
        )
    if len(declarations) > 1:
        fail("bounded fixture language permits one out-of-line module per file")
    return declarations


def validate_row_shape(row: dict[str, object]) -> None:
    if set(row) != ROW_KEYS:
        fail(f"row keys are not the registered shape: {row.get('id', '<unknown>')}")
    if row["kind"] != "module_test_boundary" or row["mode"] not in {"strict", "workspace"}:
        fail(f"invalid kind or mode in {row['id']}")
    if not isinstance(row["crate_name"], str) or not row["crate_name"]:
        fail(f"invalid crate name in {row['id']}")
    for field in ("crate_root", "workspace_root", "root_file"):
        normalized(row[field])
    if not isinstance(row["files"], dict) or not isinstance(row["symlinks"], dict):
        fail(f"file maps are not objects in {row['id']}")
    file_paths = {normalized(path) for path in row["files"]}
    link_paths = {normalized(path) for path in row["symlinks"]}
    if file_paths & link_paths:
        fail(f"file and symlink paths overlap in {row['id']}")
    if row["root_file"] not in file_paths:
        fail(f"root file is not a declared regular file in {row['id']}")
    for path, text in row["files"].items():
        normalized(path)
        if not isinstance(text, str):
            fail(f"file content is not UTF-8 text in {row['id']}")
    for path, target in row["symlinks"].items():
        normalized(path)
        if not isinstance(target, str) or target.startswith("/"):
            fail(f"invalid symlink in {row['id']}")
        physical = canonical(path, row["symlinks"])
        if physical not in file_paths:
            fail(f"symlink target is not a declared physical file in {row['id']}")
    expected = row["expected"]
    if not isinstance(expected, dict) or set(expected) != EXPECTED_KEYS:
        fail(f"invalid expected shape in {row['id']}")
    if expected["outcome"] not in {"accepted", "refused"}:
        fail(f"invalid expected outcome in {row['id']}")
    if expected["outcome"] == "refused":
        if expected["reason_contains"] not in {
            "outside crate root",
            "outside permitted test source root",
            "crate root",
        }:
            fail(f"refused reason must preserve the boundary wording in {row['id']}")
        if within(row["crate_root"], row["workspace_root"]):
            if expected["reason_contains"] not in {
                "outside crate root",
                "outside permitted test source root",
            }:
                fail(f"ordinary refusal has the wrong reason in {row['id']}")
        elif expected["reason_contains"] != "crate root":
            fail(f"invalid workspace-root configuration has the wrong reason in {row['id']}")
        if expected["modules"] or expected["test_edge"] is not None or expected["source_files"]:
            fail(f"refused row carries accepted-only data in {row['id']}")
    else:
        if not within(row["crate_root"], row["workspace_root"]):
            fail(f"accepted row has crate outside workspace root in {row['id']}")
        if expected["reason_contains"] is not None:
            fail(f"accepted row carries a refusal reason in {row['id']}")
        if not isinstance(expected["modules"], list) or not expected["modules"]:
            fail(f"accepted row has no module list in {row['id']}")
        if not isinstance(expected["test_edge"], dict) or set(expected["test_edge"]) != {"from", "to"}:
            fail(f"accepted row has no test edge in {row['id']}")
        if not isinstance(expected["source_files"], dict) or not expected["source_files"]:
            fail(f"accepted row has no source digests in {row['id']}")
        for path, value in expected["source_files"].items():
            normalized(path)
            if path not in file_paths or not SHA256.fullmatch(value):
                fail(f"invalid source digest in {row['id']}")
    mutation = row["mutation"]
    if mutation is not None:
        if expected["outcome"] != "accepted":
            fail(f"refused row carries a mutation in {row['id']}")
        if not isinstance(mutation, dict) or set(mutation) != MUTATION_KEYS:
            fail(f"invalid mutation shape in {row['id']}")
        normalized(mutation["path"])
        if mutation["path"] not in file_paths or not isinstance(mutation["text"], str):
            fail(f"mutation does not replace a declared regular file in {row['id']}")
        if not isinstance(mutation["expected_sha256"], str) or not SHA256.fullmatch(
            mutation["expected_sha256"]
        ):
            fail(f"invalid mutation digest in {row['id']}")


def evaluate(row: dict[str, object]) -> dict[str, object]:
    """Evaluate the registered layout grammar, not arbitrary Rust."""
    if not within(row["crate_root"], row["workspace_root"]):
        return {
            "outcome": "refused",
            "reason_contains": "crate root",
            "modules": [],
            "test_edge": None,
            "source_files": {},
            "target": None,
        }
    files = row["files"]
    symlinks = row["symlinks"]
    root_file = row["root_file"]
    root_text = files[root_file]
    if "pub fn root_only(){}" not in root_text:
        fail(f"root is outside the bounded hand-layout domain in {row['id']}")
    modules = [row["crate_name"]]
    traversed: list[tuple[str, str]] = [(root_file, row["crate_name"])]
    visited = {root_file}
    refusal: dict[str, object] | None = None

    def visit(physical: str, logical: str, inherited_test: bool) -> None:
        nonlocal refusal
        if refusal is not None:
            return
        declarations = parse_module_declarations(files[physical])
        for declaration in declarations:
            name = declaration["name"]
            path_attr = declaration["path"]
            if path_attr is None:
                target_link = posixpath.join(posixpath.dirname(physical), f"{name}.rs")
            else:
                target_link = posixpath.join(posixpath.dirname(physical), path_attr)
            target = canonical(target_link, symlinks)
            context_is_test = inherited_test or declaration["exact_test"]
            if not boundary_allows(
                target,
                row["crate_root"],
                row["workspace_root"],
                context_is_test,
                row["mode"],
            ):
                # Do not inspect the target before this decision. Refused
                # targets may intentionally contain invalid Rust.
                reason = "outside crate root"
                if (
                    context_is_test
                    and row["mode"] == "workspace"
                    and row["workspace_root"] is not None
                    and not within(target, row["workspace_root"])
                ):
                    reason = "outside permitted test source root"
                refusal = {
                    "outcome": "refused",
                    "reason_contains": reason,
                    "modules": [],
                    "test_edge": None,
                    "source_files": {},
                    "target": target,
                }
                return
            if target not in files:
                fail(f"accepted target is not a declared physical file in {row['id']}")
            if target in visited:
                fail(f"bounded fixture contains a repeated physical module: {row['id']}")
            visited.add(target)
            child_logical = f"{logical}::{name}"
            modules.append(child_logical)
            traversed.append((target, child_logical))
            visit(target, child_logical, context_is_test)

    visit(root_file, row["crate_name"], False)
    if refusal is not None:
        return refusal
    edge_sources = [logical for physical, logical in traversed if "pub fn shared(){crate::root_only();}" in files[physical]]
    if len(edge_sources) != 1:
        fail(f"bounded fixture must contain one helper edge in {row['id']}")
    source_files = {physical: digest(files[physical]) for physical, _ in traversed}
    return {
        "outcome": "accepted",
        "reason_contains": None,
        "modules": modules,
        "test_edge": {"from": edge_sources[0], "to": row["crate_name"]},
        "source_files": source_files,
        "target": traversed[-1][0],
    }


def verify_hand_controls(rows: list[dict[str, object]]) -> None:
    # These are direct predicate checks, independent of row expectations.
    assert not boundary_allows("workspace/shared/helper.rs", "workspace/member", "workspace", True, "strict")
    assert boundary_allows("workspace/shared/helper.rs", "workspace/member", "workspace", True, "workspace")
    assert not boundary_allows("workspace/shared/helper.rs", "workspace/member", "workspace", False, "workspace")
    assert not boundary_allows("external/helper.rs", "workspace/member", "workspace", True, "workspace")
    assert boundary_allows("workspace/member/src/lib.rs", "workspace/member", "workspace", False, "strict")
    assert not boundary_allows("workspace/shared/helper.rs", "workspace/member", None, True, "workspace")
    assert not within("workspace/member", "workspace/shared")

    by_id = {row["id"]: row for row in rows}
    nested = evaluate(by_id["nested_out_of_line_inherited_cfg_test_helper"])
    if nested["outcome"] != "accepted" or nested["modules"] != [
        "x",
        "x::checks",
        "x::checks::shared",
    ]:
        fail("out-of-line inherited test control did not traverse all three modules")
    checks_file = by_id["nested_out_of_line_inherited_cfg_test_helper"]["files"][
        "workspace/member/src/checks.rs"
    ]
    if "#[cfg(test)]" in checks_file:
        fail("out-of-line nested control added an inner cfg(test)")

    for case_id in (
        "workspace_external_inner_cfg_test_non_test_declaration",
        "workspace_external_nonexact_cfg_test_declaration",
        "workspace_external_cfg_attr_path_declaration",
    ):
        result = evaluate(by_id[case_id])
        if result["outcome"] != "refused":
            fail(f"non-exact declaration control was allowed: {case_id}")
    parsed_any = parse_module_declarations(
        by_id["workspace_external_nonexact_cfg_test_declaration"]["files"]["workspace/member/src/lib.rs"]
    )
    parsed_attr = parse_module_declarations(
        by_id["workspace_external_cfg_attr_path_declaration"]["files"]["workspace/member/src/lib.rs"]
    )
    if parsed_any[0]["exact_test"] or parsed_attr[0]["exact_test"]:
        fail("non-exact cfg or cfg_attr was mistaken for exact cfg(test)")
    if "#![cfg(test)]" not in by_id["workspace_external_inner_cfg_test_non_test_declaration"]["files"][
        "workspace/shared/helper.rs"
    ]:
        fail("inner-attribute negative control lost its marker")

    inside = by_id["workspace_cfg_test_canonical_symlink_inside"]
    outside = by_id["workspace_cfg_test_canonical_symlink_outside"]
    if evaluate(inside)["target"] != "workspace/shared/helper.rs":
        fail("inside symlink did not bind to its canonical physical target")
    outside_result = evaluate(outside)
    if outside_result["target"] != "external/helper.rs":
        fail("outside symlink did not bind to its canonical physical target")
    if outside_result["reason_contains"] != "outside permitted test source root":
        fail("workspace test-root symlink escape lost its distinct diagnostic")
    direct_result = evaluate(by_id["workspace_cfg_test_target_outside_workspace"])
    if direct_result["reason_contains"] != "outside permitted test source root":
        fail("workspace test-root direct escape lost its distinct diagnostic")
    if evaluate(by_id["workspace_production_external_helper"])["reason_contains"] != "outside crate root":
        fail("production escape changed its crate-root diagnostic")
    if evaluate(by_id["workspace_external_nonexact_cfg_test_declaration"])["reason_contains"] != "outside crate root":
        fail("non-exact test condition changed its crate-root diagnostic")
    if evaluate(by_id["workspace_root_narrower_than_crate_root"])["reason_contains"] != "crate root":
        fail("narrow workspace-root configuration was not rejected first")


def verify_cases() -> list[dict[str, object]]:
    try:
        rows = json.loads((PACKAGE / "CASES.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"cannot read CASES.json: {exc}")
    if not isinstance(rows, list) or len(rows) != 12:
        fail("fixture must contain exactly twelve bounded cases")
    seen: set[str] = set()
    for row in rows:
        if not isinstance(row, dict):
            fail("case row is not an object")
        validate_row_shape(row)
        if row["id"] in seen:
            fail(f"duplicate case id: {row['id']}")
        seen.add(row["id"])
        result = evaluate(row)
        expected = row["expected"]
        comparable = {key: result[key] for key in EXPECTED_KEYS}
        if comparable != expected:
            fail(f"hand-derived result mismatch in {row['id']}")
        if result["outcome"] == "refused":
            target = result["target"]
            if row["id"] in INVALID_REFUSAL_IDS:
                if target is None or "intentionally invalid Rust" not in row["files"][target]:
                    fail(f"refusal lacks its parse-order discriminant in {row['id']}")
        mutation = row["mutation"]
        if mutation is not None:
            mutated = dict(row)
            mutated["files"] = dict(row["files"])
            mutated["files"][mutation["path"]] = mutation["text"]
            mutated_result = evaluate(mutated)
            if mutated_result["outcome"] != "accepted" or mutated_result["test_edge"] != result["test_edge"]:
                fail(f"mutation changed the bounded module edge in {row['id']}")
            if mutated_result["modules"] != result["modules"]:
                fail(f"mutation changed the bounded module list in {row['id']}")
            if mutated_result["source_files"][mutation["path"]] != mutation["expected_sha256"]:
                fail(f"mutation digest does not match in {row['id']}")
            if mutated_result["source_files"][mutation["path"]] == result["source_files"][mutation["path"]]:
                fail(f"mutation did not change the bound digest in {row['id']}")
    verify_hand_controls(rows)
    return rows


def verify_package() -> list[dict[str, object]]:
    actual_files = sorted(path.name for path in PACKAGE.iterdir() if path.is_file())
    if actual_files != sorted(ALL_FILES):
        fail(f"package file set is {actual_files!r}, expected exactly {list(ALL_FILES)!r}")
    try:
        registration = tomllib.loads((PACKAGE / "registration.toml").read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        fail(f"invalid registration.toml: {exc}")
    expected_top = {
        "schema_version": 1,
        "id": "module-test-boundary",
        "kind": "module_test_boundary",
        "payloads": 7,
        "files": 9,
        "actual_case_count": 12,
        "authored": True,
        "prng": False,
        "model": "gpt-5.6-luna",
        "effort": "high",
        "kind_counts": {"module_test_boundary": 12},
    }
    for key, value in expected_top.items():
        if registration.get(key) != value:
            fail(f"registration metadata mismatch at {key}")
    if set(registration) != set(expected_top) | {"content_sha256", "source_snapshots"}:
        fail("registration has missing or invented metadata fields")
    if registration["content_sha256"] != digest_bytes((PACKAGE / "SHA256SUMS").read_bytes()):
        fail("registration content_sha256 does not measure SHA256SUMS bytes")
    expected_snapshots = []
    for relative_path, documentary_path in SNAPSHOT_SPEC:
        expected_snapshots.append(
            {
                "relative_path": relative_path,
                "original_documentary_path": documentary_path,
                "sha256": digest_bytes((PACKAGE / relative_path).read_bytes()),
            }
        )
    if registration["source_snapshots"] != expected_snapshots:
        fail("registration source snapshots are not self-contained measured metadata")
    try:
        lines = (PACKAGE / "SHA256SUMS").read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        fail(f"cannot read SHA256SUMS: {exc}")
    manifest: dict[str, str] = {}
    for line in lines:
        parts = line.split("  ")
        if len(parts) != 2 or not SHA256.fullmatch(parts[0]) or parts[1] in manifest:
            fail("malformed SHA256SUMS")
        manifest[parts[1]] = parts[0]
    if set(manifest) != set(PAYLOADS):
        fail("SHA256SUMS must cover exactly the seven payloads")
    for name in PAYLOADS:
        if digest_bytes((PACKAGE / name).read_bytes()) != manifest[name]:
            fail(f"payload digest mismatch: {name}")
    rows = verify_cases()
    if len(rows) != registration["actual_case_count"]:
        fail("registration actual_case_count disagrees with CASES.json")
    return rows


def reproduce(destination: str) -> None:
    verify_package()
    frozen_inputs = {name: (PACKAGE / name).read_bytes() for name in ALL_FILES}
    target = (PACKAGE / destination).resolve()
    if target.parent != PACKAGE or target == PACKAGE or os.path.lexists(target):
        fail("reproduction destination must be a fresh direct child of the package")
    target.mkdir()
    try:
        for name, data in frozen_inputs.items():
            (target / name).write_bytes(data)
        if sorted(path.name for path in target.iterdir() if path.is_file()) != sorted(ALL_FILES):
            fail("reproduction did not create exactly nine files")
        for name, data in frozen_inputs.items():
            if (target / name).read_bytes() != data:
                fail(f"reproduction differs for {name}")
    except Exception:
        shutil.rmtree(target)
        raise
    print(f"module-test-boundary: reproduced byte-identical package at {target}")


def main(argv: list[str]) -> int:
    try:
        if len(argv) == 1:
            rows = verify_package()
            print(f"module-test-boundary: PASS cases={len(rows)} payloads=7 files=9")
            return 0
        if len(argv) == 3 and argv[1] == "--reproduce":
            reproduce(argv[2])
            return 0
        fail("usage: reference.py [--reproduce FRESH_DIRECT_CHILD]")
    except (FixtureError, AssertionError, OSError, ValueError) as exc:
        print(f"module-test-boundary: FAIL: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
