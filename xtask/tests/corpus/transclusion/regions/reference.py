#!/usr/bin/env python3
"""Independent deterministic reference and fixture generator for region graphs.

The default invocation is read-only.  ``--reproduce-to`` creates a complete
package copy beneath the script's own directory, using only frozen snapshots
and this reference.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import random
import shutil
import sys
import tomllib
import unicodedata
from pathlib import Path
from typing import Any


SEED = 90092026
RANDOM_CASES = 256
KIND = "region_graph"
MODEL = "gpt-5.6-luna"
EFFORT = "high"
EXPECTED_SNAPSHOT_PATHS = {
    "contract": "xtask/tests/corpus/transclusion/regions/source-snapshots/contract.md",
    "bdr": "xtask/tests/corpus/transclusion/regions/source-snapshots/bdr.md",
    "prompt": "xtask/tests/corpus/transclusion/regions/source-snapshots/prompt.md",
}
ORIGINAL_SNAPSHOT_PATHS = {
    "contract": "docs/architecture/transclusion-contract.md",
    "bdr": "docs/adr/BDR-0007-site-owns-page-tree.md",
    "prompt": "xtask/tests/corpus/write-prompts/CHG-008/pd-transclusion-regions-generator-prompt.md",
}


Region = tuple[str, str | None]
Edge = tuple[Region, int, Region]


def region(page: str, anchor: str | None = None) -> dict[str, Any]:
    return {"page": page, "anchor": anchor}


def region_tuple(value: dict[str, Any]) -> Region:
    if not isinstance(value, dict) or set(value) != {"page", "anchor"}:
        raise AssertionError(f"invalid region shape: {value!r}")
    page = value["page"]
    anchor = value["anchor"]
    if not isinstance(page, str):
        raise AssertionError("region page must be a string")
    if anchor is not None and (not isinstance(anchor, str) or not anchor):
        raise AssertionError("region anchors must be None or nonempty strings")
    return page, anchor


def region_key(value: Region) -> tuple[str, int, str]:
    page, anchor = value
    return page, 0 if anchor is None else 1, "" if anchor is None else anchor


def region_json(value: Region) -> dict[str, Any]:
    return region(value[0], value[1])


def edge_tuple(value: dict[str, Any]) -> Edge:
    if not isinstance(value, dict) or set(value) != {"source", "transclusion_id", "target"}:
        raise AssertionError(f"invalid edge shape: {value!r}")
    source = region_tuple(value["source"])
    identifier = value["transclusion_id"]
    if isinstance(identifier, bool) or not isinstance(identifier, int) or identifier < 0:
        raise AssertionError("transclusion_id must be a nonnegative integer")
    target = region_tuple(value["target"])
    return source, identifier, target


def edge_json(value: Edge) -> dict[str, Any]:
    source, identifier, target = value
    return {
        "source": region_json(source),
        "transclusion_id": identifier,
        "target": region_json(target),
    }


def canonical_cycle(path: list[Region]) -> tuple[Region, ...]:
    if len(path) < 2 or path[0] != path[-1]:
        raise AssertionError(f"cycle is not closed: {path!r}")
    body = path[:-1]
    rotations = [body[offset:] + body[:offset] for offset in range(len(body))]
    chosen = min(rotations, key=lambda candidate: tuple(region_key(item) for item in candidate))
    return tuple(chosen + [chosen[0]])


def graph_parts(case: dict[str, Any]) -> tuple[list[Region], list[Edge]]:
    if case.get("kind") != KIND:
        raise AssertionError(f"unexpected case kind: {case.get('kind')!r}")
    vertices_raw = case.get("vertices")
    edges_raw = case.get("edges")
    if not isinstance(vertices_raw, list) or not isinstance(edges_raw, list):
        raise AssertionError("vertices and edges must be arrays")
    vertices = [region_tuple(item) for item in vertices_raw]
    if len(set(vertices)) != len(vertices):
        raise AssertionError("vertices must be unique")
    vertex_set = set(vertices)
    edges = [edge_tuple(item) for item in edges_raw]
    occurrence_keys: set[tuple[Region, int]] = set()
    for source, identifier, target in edges:
        if source not in vertex_set or target not in vertex_set:
            raise AssertionError("edge endpoint is not a vertex")
        occurrence = source, identifier
        if occurrence in occurrence_keys:
            raise AssertionError("source/transclusion_id occurs more than once")
        occurrence_keys.add(occurrence)
    return vertices, edges


def compute_expected(case: dict[str, Any]) -> dict[str, Any]:
    vertices, edges = graph_parts(case)
    adjacency: dict[Region, list[tuple[int, Region]]] = {item: [] for item in vertices}
    for source, identifier, target in edges:
        adjacency[source].append((identifier, target))
    for outgoing in adjacency.values():
        outgoing.sort(key=lambda item: (item[0], region_key(item[1])))

    visited: set[Region] = set()
    active: list[Region] = []
    blocked: list[tuple[Region, int, tuple[Region, ...]]] = []
    cycles: set[tuple[Region, ...]] = set()

    def visit(current: Region) -> None:
        visited.add(current)
        active.append(current)
        for identifier, target in adjacency[current]:
            if target in active:
                suffix = active[active.index(target) :] + [target]
                witness = canonical_cycle(suffix)
                blocked.append((current, identifier, witness))
                cycles.add(witness)
            elif target not in visited:
                visit(target)
        active.pop()

    for root in sorted(vertices, key=region_key):
        if root not in visited:
            visit(root)

    blocked.sort(key=lambda item: (region_key(item[0]), item[1]))
    cycle_list = sorted(cycles, key=lambda path: tuple(region_key(item) for item in path))
    return {
        "blocked": [
            {
                "source": region_json(source),
                "transclusion_id": identifier,
                "path": [region_json(item) for item in path],
            }
            for source, identifier, path in blocked
        ],
        "cycles": [[region_json(item) for item in path] for path in cycle_list],
        "remaining_is_dag": True,
    }


def remaining_is_dag(vertices: list[Region], edges: list[Edge], blocked: set[tuple[Region, int]]) -> bool:
    adjacency: dict[Region, list[Region]] = {item: [] for item in vertices}
    for source, identifier, target in edges:
        if (source, identifier) not in blocked:
            adjacency[source].append(target)
    state: dict[Region, int] = {}

    def visit(current: Region) -> bool:
        state[current] = 1
        for target in adjacency[current]:
            if state.get(target, 0) == 1:
                return False
            if state.get(target, 0) == 0 and not visit(target):
                return False
        state[current] = 2
        return True

    return all(visit(root) for root in sorted(vertices, key=region_key) if state.get(root, 0) == 0)


def validate_expected(case: dict[str, Any]) -> None:
    vertices, edges = graph_parts(case)
    expected = case.get("expected")
    if not isinstance(expected, dict):
        raise AssertionError("missing expected object")
    actual = compute_expected(case)
    if expected != actual:
        raise AssertionError(f"expected mismatch for {case.get('id')}: {expected!r} != {actual!r}")

    edge_targets = {(source, identifier): target for source, identifier, target in edges}
    blocked_keys: set[tuple[Region, int]] = set()
    for item in expected["blocked"]:
        source = region_tuple(item["source"])
        identifier = item["transclusion_id"]
        path = [region_tuple(value) for value in item["path"]]
        key = source, identifier
        if key in blocked_keys:
            raise AssertionError("blocked occurrence repeated")
        blocked_keys.add(key)
        if key not in edge_targets:
            raise AssertionError("blocked occurrence is not an edge")
        if path[0] != path[-1] or canonical_cycle(path) != tuple(path):
            raise AssertionError("blocked witness is not a canonical closed path")
        if not any(path[index] == source and path[index + 1] == edge_targets[key] for index in range(len(path) - 1)):
            raise AssertionError("blocked edge is not part of its witness")

    cycle_tuples: list[tuple[Region, ...]] = []
    for path_raw in expected["cycles"]:
        path = tuple(region_tuple(value) for value in path_raw)
        if len(path) < 2 or path[0] != path[-1] or canonical_cycle(list(path)) != path:
            raise AssertionError("cycle is not a canonical closed path")
        for index in range(len(path) - 1):
            if not any(source == path[index] and target == path[index + 1] for source, _, target in edges):
                raise AssertionError("cycle contains a missing graph edge")
        cycle_tuples.append(path)
    if cycle_tuples != sorted(set(cycle_tuples), key=lambda path: tuple(region_key(item) for item in path)):
        raise AssertionError("cycles are not sorted and deduplicated")
    if expected["remaining_is_dag"] is not True or not remaining_is_dag(vertices, edges, blocked_keys):
        raise AssertionError("blocked graph is not a DAG")


def permutation_check(case: dict[str, Any], ordinal: int) -> None:
    shuffled = copy.deepcopy(case)
    rng = random.Random(SEED ^ (ordinal + 0x5EED))
    rng.shuffle(shuffled["vertices"])
    rng.shuffle(shuffled["edges"])
    if compute_expected(shuffled) != case["expected"]:
        raise AssertionError(f"permutation changed output for {case.get('id')}")


def authored_cases() -> list[dict[str, Any]]:
    cases: list[dict[str, Any]] = []

    def add(identifier: str, vertices: list[dict[str, Any]], edges: list[dict[str, Any]]) -> None:
        cases.append({"id": identifier, "kind": KIND, "vertices": vertices, "edges": edges})

    add("control-empty", [], [])
    add(
        "control-same-page-acyclic",
        [region("same-page", "one"), region("same-page", "two"), region("same-page", "isolated")],
        [
            {
                "source": region("same-page", "one"),
                "transclusion_id": 7,
                "target": region("same-page", "two"),
            }
        ],
    )
    add(
        "control-self-loop",
        [region("alpha")],
        [{"source": region("alpha"), "transclusion_id": 0, "target": region("alpha")}],
    )
    add(
        "control-two-region-cycle",
        [region("cycle-a"), region("cycle-b")],
        [
            {"source": region("cycle-a"), "transclusion_id": 0, "target": region("cycle-b")},
            {"source": region("cycle-b"), "transclusion_id": 0, "target": region("cycle-a")},
        ],
    )
    add(
        "control-diamond-multiple-back-edges",
        [region("scc", "a"), region("scc", "b"), region("scc", "c"), region("scc", "d")],
        [
            {"source": region("scc", "a"), "transclusion_id": 0, "target": region("scc", "b")},
            {"source": region("scc", "a"), "transclusion_id": 1, "target": region("scc", "c")},
            {"source": region("scc", "b"), "transclusion_id": 0, "target": region("scc", "d")},
            {"source": region("scc", "c"), "transclusion_id": 0, "target": region("scc", "d")},
            {"source": region("scc", "d"), "transclusion_id": 0, "target": region("scc", "b")},
            {"source": region("scc", "d"), "transclusion_id": 1, "target": region("scc", "a")},
        ],
    )
    add(
        "control-disconnected-unicode-scc",
        [
            region("café", "résumé"),
            region("東京", "入口"),
            region("東京", "出口"),
            region("孤立"),
        ],
        [
            {"source": region("café", "résumé"), "transclusion_id": 4, "target": region("café", "résumé")},
            {"source": region("東京", "入口"), "transclusion_id": 0, "target": region("東京", "出口")},
            {"source": region("東京", "出口"), "transclusion_id": 3, "target": region("東京", "入口")},
        ],
    )
    return cases


def random_case(ordinal: int) -> dict[str, Any]:
    rng = random.Random(SEED + ordinal * 1009)
    vertex_count = rng.randint(1, 16)
    vertices: list[dict[str, Any]] = []
    for index in range(vertex_count):
        page = f"random-{ordinal:03d}-{index // 3}"
        anchor = None if index % 3 == 0 else f"region-{index}"
        vertices.append(region(page, anchor))

    edge_values: list[dict[str, Any]] = []
    vertex_values = [region_tuple(item) for item in vertices]
    for source_index, source in enumerate(vertex_values):
        degree = rng.randint(0, min(4, vertex_count))
        target_indices = rng.sample(range(vertex_count), degree)
        identifiers = rng.sample(range(0, 64), degree)
        for identifier, target_index in zip(identifiers, target_indices):
            edge_values.append(
                {"source": region_json(source), "transclusion_id": identifier, "target": region_json(vertex_values[target_index])}
            )
    rng.shuffle(vertices)
    rng.shuffle(edge_values)
    return {"id": f"random-{ordinal:03d}", "kind": KIND, "vertices": vertices, "edges": edge_values}


def generated_cases() -> list[dict[str, Any]]:
    cases = authored_cases() + [random_case(ordinal) for ordinal in range(RANDOM_CASES)]
    for case in cases:
        case["expected"] = compute_expected(case)
    return cases


def cases_bytes(cases: list[dict[str, Any]]) -> bytes:
    return (json.dumps(cases, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def payload_paths(root: Path) -> list[Path]:
    paths = [
        root / "CASES.json",
        root / "README.md",
        root / "reference.py",
        root / "selftestreport.txt",
        root / "source-snapshots" / "bdr.md",
        root / "source-snapshots" / "contract.md",
        root / "source-snapshots" / "prompt.md",
    ]
    return sorted(paths, key=lambda path: path.relative_to(root).as_posix())


def selftest_report(cases: list[dict[str, Any]], case_data: bytes) -> bytes:
    authored_count = len(authored_cases())
    lines = [
        "transclusion-region-corpus-selftest",
        "status = passed",
        f"seed = {SEED}",
        f"authored_cases = {authored_count}",
        f"random_cases = {RANDOM_CASES}",
        f"case_count = {len(cases)}",
        "kind_count = 1",
        "payload_count = 7",
        "checks = hand_controls,graph_invariants,permutation_invariance,byte_reproduction",
        f"cases_sha256 = {sha256_bytes(case_data)}",
    ]
    return ("\n".join(lines) + "\n").encode("utf-8")


def registration_bytes(cases: list[dict[str, Any]], sums_data: bytes, root: Path) -> bytes:
    source_rows = []
    for name in ("contract", "bdr", "prompt"):
        path = root / Path(EXPECTED_SNAPSHOT_PATHS[name]).relative_to("xtask/tests/corpus/transclusion/regions")
        source_rows.append(
            (
                name,
                EXPECTED_SNAPSHOT_PATHS[name],
                ORIGINAL_SNAPSHOT_PATHS[name],
                sha256_file(path),
            )
        )
    lines = [
        "schema_version = 1",
        f"seed = {SEED}",
        f'model = "{MODEL}"',
        f'effort = "{EFFORT}"',
        f'content_sha256 = "{sha256_bytes(sums_data)}"',
        "",
        "[counts]",
        f"cases = {len(cases)}",
        "kinds = 1",
        "payloads = 7",
        f"authored = {len(authored_cases())}",
        f"random = {RANDOM_CASES}",
        "",
        "[kinds]",
        f"region_graph = {len(cases)}",
    ]
    for name, path, original, digest in source_rows:
        lines.extend(
            [
                "",
                "[[sources]]",
                f'name = "{name}"',
                f'path = "{path}"',
                f'original_path = "{original}"',
                f'sha256 = "{digest}"',
            ]
        )
    return ("\n".join(lines) + "\n").encode("utf-8")


def sums_bytes(root: Path) -> bytes:
    lines = [f"{sha256_file(path)}  {path.relative_to(root).as_posix()}" for path in payload_paths(root)]
    return ("\n".join(lines) + "\n").encode("utf-8")


def verify_sums(root: Path) -> list[str]:
    raw = (root / "SHA256SUMS").read_text(encoding="utf-8")
    entries: list[tuple[str, str]] = []
    for line in raw.splitlines():
        digest, relative = line.split("  ", 1)
        entries.append((digest, relative))
    relative_paths = [relative for _, relative in entries]
    expected_paths = [path.relative_to(root).as_posix() for path in payload_paths(root)]
    if relative_paths != expected_paths:
        raise AssertionError("SHA256SUMS is not the sorted complete payload list")
    for digest, relative in entries:
        if digest != sha256_file(root / relative):
            raise AssertionError(f"payload digest mismatch: {relative}")
    return relative_paths


def validate_cases(cases: list[dict[str, Any]]) -> None:
    if len(cases) != len(authored_cases()) + RANDOM_CASES:
        raise AssertionError("case count mismatch")
    if [case["id"] for case in cases] != [case["id"] for case in generated_cases()]:
        raise AssertionError("case ids are not deterministic")
    for ordinal, case in enumerate(cases):
        validate_expected(case)
        permutation_check(case, ordinal)
    hand = {case["id"]: case for case in cases if case["id"].startswith("control-")}
    expected_hand = {
        "control-empty": {"blocked": [], "cycles": [], "remaining_is_dag": True},
        "control-self-loop": {
            "blocked": [{"source": region("alpha"), "transclusion_id": 0, "path": [region("alpha"), region("alpha")]}],
            "cycles": [[region("alpha"), region("alpha")]],
            "remaining_is_dag": True,
        },
        "control-two-region-cycle": {
            "blocked": [
                {"source": region("cycle-b"), "transclusion_id": 0, "path": [region("cycle-a"), region("cycle-b"), region("cycle-a")]}
            ],
            "cycles": [[region("cycle-a"), region("cycle-b"), region("cycle-a")]],
            "remaining_is_dag": True,
        },
        "control-same-page-acyclic": {"blocked": [], "cycles": [], "remaining_is_dag": True},
        "control-diamond-multiple-back-edges": {
            "blocked": [
                {
                    "source": region("scc", "d"),
                    "transclusion_id": 0,
                    "path": [region("scc", "b"), region("scc", "d"), region("scc", "b")],
                },
                {
                    "source": region("scc", "d"),
                    "transclusion_id": 1,
                    "path": [region("scc", "a"), region("scc", "b"), region("scc", "d"), region("scc", "a")],
                },
            ],
            "cycles": [
                [region("scc", "a"), region("scc", "b"), region("scc", "d"), region("scc", "a")],
                [region("scc", "b"), region("scc", "d"), region("scc", "b")],
            ],
            "remaining_is_dag": True,
        },
    }
    for identifier, expected in expected_hand.items():
        if hand[identifier]["expected"] != expected:
            raise AssertionError(f"hand-checked control changed: {identifier}")
    for case in cases:
        if case["id"] == "control-disconnected-unicode-scc":
            for vertex in case["vertices"]:
                if unicodedata.normalize("NFC", vertex["page"]) != vertex["page"]:
                    raise AssertionError("authored Unicode page is not NFC")


def verify_package(root: Path) -> None:
    expected_files = sorted(
        [path.relative_to(root).as_posix() for path in payload_paths(root)]
        + ["SHA256SUMS", "registration.toml"]
    )
    actual_files = sorted(path.relative_to(root).as_posix() for path in root.rglob("*") if path.is_file())
    if actual_files != expected_files:
        raise AssertionError(f"package must contain exactly nine files: {actual_files!r}")
    cases_data = (root / "CASES.json").read_bytes()
    cases = json.loads(cases_data.decode("utf-8"))
    validate_cases(cases)
    verify_sums(root)
    registration = tomllib.loads((root / "registration.toml").read_text(encoding="utf-8"))
    if registration["seed"] != SEED or registration["model"] != MODEL or registration["effort"] != EFFORT:
        raise AssertionError("registration identity mismatch")
    if registration["content_sha256"] != sha256_file(root / "SHA256SUMS"):
        raise AssertionError("registration content digest mismatch")
    counts = registration["counts"]
    if counts != {
        "cases": len(cases),
        "kinds": 1,
        "payloads": len(payload_paths(root)),
        "authored": len(authored_cases()),
        "random": RANDOM_CASES,
    }:
        raise AssertionError("registration counts mismatch")
    source_rows = {row["name"]: row for row in registration["sources"]}
    for name, snapshot_path in EXPECTED_SNAPSHOT_PATHS.items():
        snapshot = root / Path(snapshot_path).relative_to("xtask/tests/corpus/transclusion/regions")
        row = source_rows[name]
        if row["path"] != snapshot_path or row["sha256"] != sha256_file(snapshot):
            raise AssertionError(f"source metadata mismatch: {name}")
    expected_report = selftest_report(cases, cases_data)
    if (root / "selftestreport.txt").read_bytes() != expected_report:
        raise AssertionError("selftest report mismatch")


def ensure_output_inside_package(package: Path, output: Path) -> None:
    package = package.resolve()
    output = output.resolve()
    if output.parent != package:
        raise ValueError("--reproduce-to must name a fresh direct child of the script package")
    if output.exists() and any(output.iterdir()):
        raise ValueError("--reproduce-to must be empty")
    output.mkdir(parents=True, exist_ok=True)


def reproduce(package: Path, output: Path) -> None:
    ensure_output_inside_package(package, output)
    cases = generated_cases()
    case_data = cases_bytes(cases)
    (output / "CASES.json").write_bytes(case_data)
    shutil.copyfile(package / "README.md", output / "README.md")
    shutil.copyfile(package / "reference.py", output / "reference.py")
    (output / "source-snapshots").mkdir()
    for snapshot_path in EXPECTED_SNAPSHOT_PATHS.values():
        relative = Path(snapshot_path).relative_to("xtask/tests/corpus/transclusion/regions")
        shutil.copyfile(package / relative, output / relative)
    (output / "selftestreport.txt").write_bytes(selftest_report(cases, case_data))
    sums_data = sums_bytes(output)
    (output / "SHA256SUMS").write_bytes(sums_data)
    (output / "registration.toml").write_bytes(registration_bytes(cases, sums_data, output))
    verify_package(output)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reproduce-to", type=Path, help="write a reproducible package subdirectory")
    args = parser.parse_args()
    package = Path(__file__).resolve().parent
    if args.reproduce_to is None:
        verify_package(package)
        print(f"self-tests passed: {len(json.loads((package / 'CASES.json').read_text(encoding='utf-8')))} cases")
    else:
        reproduce(package, args.reproduce_to)
        print(f"reproduced package: {args.reproduce_to.resolve()}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, OSError, ValueError, KeyError, tomllib.TOMLDecodeError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        raise SystemExit(1)
