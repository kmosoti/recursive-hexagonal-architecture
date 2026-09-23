#!/usr/bin/env python3
"""Verify the no-std graph experiment is only the approved source change."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


REPO_ROOT = Path(__file__).resolve().parents[2]
EXPERIMENT_ROOT = REPO_ROOT / "experiments" / "no-std-graph"
SNAPSHOT_PATH = EXPERIMENT_ROOT / "src" / "lib.rs"
SOURCES_PATH = EXPERIMENT_ROOT / "sources.sha256"
BASELINE_PATH = EXPERIMENT_ROOT / "baseline-revision.txt"

SOURCE_KEYS = (
    "crates/graph/src/lib.rs",
    "crates/graph/tests/oracle_resolve.rs",
    "crates/graph/tests/properties.rs",
)
SOURCE_LINE = re.compile(r"^([0-9a-f]{64})  (.+)$")
BASELINE_LINE = re.compile(r"^([0-9a-f]{40})\n?$")

NEGATIVE_STD_PROBE = (
    b'\n#[cfg(feature = "negative-std-probe")]\n'
    b"fn negative_std_probe() {\n"
    b"    let _: std::collections::BTreeMap<(), ()> = std::collections::BTreeMap::new();\n"
    b"}\n"
)


class VerificationError(Exception):
    """A deliberate verification failure."""


def read_bytes(path: Path, label: str) -> bytes:
    try:
        return path.read_bytes()
    except OSError as exc:
        raise VerificationError(f"cannot read {label}: {exc}") from exc


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def parse_sources() -> dict[str, str]:
    raw = read_bytes(SOURCES_PATH, "sources.sha256")
    try:
        text = raw.decode("ascii")
    except UnicodeDecodeError as exc:
        raise VerificationError("sources.sha256 is not ASCII") from exc

    if not text.endswith("\n"):
        raise VerificationError("sources.sha256 must end with a newline")
    lines = text[:-1].split("\n")
    if len(lines) != 3:
        raise VerificationError("sources.sha256 must contain exactly three lines")

    parsed: dict[str, str] = {}
    for line in lines:
        match = SOURCE_LINE.fullmatch(line)
        if match is None:
            raise VerificationError(f"invalid sources.sha256 line: {line!r}")
        digest, path = match.groups()
        if path not in SOURCE_KEYS:
            raise VerificationError(f"unexpected source path: {path!r}")
        if path in parsed:
            raise VerificationError(f"duplicate source path: {path!r}")
        parsed[path] = digest

    if set(parsed) != set(SOURCE_KEYS):
        raise VerificationError("sources.sha256 does not contain the exact three paths")
    return parsed


def parse_baseline() -> str:
    raw = read_bytes(BASELINE_PATH, "baseline-revision.txt")
    try:
        text = raw.decode("ascii")
    except UnicodeDecodeError as exc:
        raise VerificationError("baseline-revision.txt is not ASCII") from exc
    match = BASELINE_LINE.fullmatch(text)
    if match is None:
        raise VerificationError("baseline-revision.txt must contain one full 40-hex Git SHA")
    return match.group(1)


def replace_once(body: bytes, old: bytes, new: bytes, label: str) -> bytes:
    count = body.count(old)
    if count != 1:
        raise VerificationError(f"expected exactly one {label}, found {count}")
    return body.replace(old, new, 1)


def reconstruct_snapshot(snapshot: bytes) -> bytes:
    body = replace_once(snapshot, b"#![no_std]\n", b"", "#![no_std] marker")
    body = replace_once(body, b"extern crate alloc;\n", b"", "extern crate alloc marker")
    body = replace_once(
        body,
        b"use alloc::collections::{BTreeMap, BTreeSet};",
        b"use std::collections::{BTreeMap, BTreeSet};",
        "alloc collections import",
    )
    body = replace_once(body, b"use alloc::{string::String, vec::Vec};\n", b"", "alloc type imports")
    return replace_once(body, NEGATIVE_STD_PROBE, b"", "negative std probe block")


def verify_git_objects(baseline: str, expected: dict[str, str]) -> dict[str, bool]:
    checked: dict[str, bool] = {}
    for path in SOURCE_KEYS:
        object_name = f"{baseline}:{path}"
        try:
            result = subprocess.run(
                ["git", "-C", str(REPO_ROOT), "cat-file", "blob", object_name],
                stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL,
                check=False,
            )
        except OSError:
            result = None

        if result is None or result.returncode != 0:
            checked[path] = False
            print(f"notverified-at-git: {path}", file=sys.stderr)
            continue

        digest = sha256(result.stdout)
        if digest != expected[path]:
            raise VerificationError(
                f"Git baseline object digest mismatch for {path}: {digest} != {expected[path]}"
            )
        checked[path] = True
    return checked


def main() -> int:
    expected = parse_sources()
    baseline = parse_baseline()

    snapshot = read_bytes(SNAPSHOT_PATH, "experiment src/lib.rs")
    reconstructed = reconstruct_snapshot(snapshot)
    reconstructed_digest = sha256(reconstructed)
    if reconstructed_digest != expected["crates/graph/src/lib.rs"]:
        raise VerificationError(
            "reconstructed graph source digest mismatch: "
            f"{reconstructed_digest} != {expected['crates/graph/src/lib.rs']}"
        )

    test_digests: dict[str, str] = {}
    for key in SOURCE_KEYS[1:]:
        experiment_path = EXPERIMENT_ROOT / "tests" / Path(key).name
        digest = sha256(read_bytes(experiment_path, f"experiment {experiment_path.name}"))
        test_digests[key] = digest
        if digest != expected[key]:
            raise VerificationError(f"test digest mismatch for {key}: {digest} != {expected[key]}")

    git_object_checked = verify_git_objects(baseline, expected)
    report = {
        "baseline_revision": baseline,
        "digests": {
            "reconstructed_graph_source": reconstructed_digest,
            "tests": test_digests,
        },
        "git_object_checked": git_object_checked,
    }
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except VerificationError as exc:
        print(f"verify_snapshot.py: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc
