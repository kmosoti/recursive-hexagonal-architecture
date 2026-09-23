#!/usr/bin/env python3
"""Generate or check the compile-time transclusion fixture mapping."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys


HERE = Path(__file__).resolve().parent
CORPUS = HERE.parent / "corpus" / "transclusion"
SPEC_SOURCE = HERE / "registered_package.rs"
OUTPUT = HERE / "registered_package_embedded.rs"
PACKAGES = ("regions", "sections", "sites", "uris")
FILES = (
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
PAYLOADS = tuple(path for path in FILES if path not in {"SHA256SUMS", "registration.toml"})

SPEC_PATTERN = re.compile(
    r'"(?P<package>regions|sections|sites|uris)"\s*=>\s*Spec\s*\{\s*'
    r'cases:\s*"(?P<cases>[0-9a-f]{64})",\s*'
    r'registration:\s*"(?P<registration>[0-9a-f]{64})",\s*'
    r'sums:\s*"(?P<sums>[0-9a-f]{64})",\s*'
    r'count:\s*(?P<count>[0-9]+),\s*'
    r'kind:\s*"(?P<kind>[^"]+)",',
    re.DOTALL,
)


def fail(message):
    raise ValueError(message)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def safe_relative_path(relative):
    parts = relative.split("/")
    return (
        bool(relative)
        and "\\" not in relative
        and all(part not in ("", ".", "..") for part in parts)
    )


def read_specs():
    matches = list(SPEC_PATTERN.finditer(SPEC_SOURCE.read_text(encoding="utf-8")))
    specs = {match["package"]: match.groupdict() for match in matches}
    if set(specs) != set(PACKAGES):
        fail(f"Spec package set differs: {sorted(specs)}")
    for package in PACKAGES:
        specs[package]["count"] = int(specs[package]["count"])
    return specs


def inventory(root):
    if root.is_symlink() or not root.is_dir():
        fail(f"package root is not a durable directory: {root}")

    actual = set()
    for directory, directories, filenames in os.walk(root, topdown=True, followlinks=False):
        for name in list(directories) + list(filenames):
            path = Path(directory) / name
            relative = path.relative_to(root).as_posix()
            if not safe_relative_path(relative):
                fail(f"unsafe package path: {relative}")
            if path.is_symlink():
                fail(f"symlink in package inventory: {path}")
            mode = path.lstat().st_mode
            if stat.S_ISDIR(mode):
                continue
            if not stat.S_ISREG(mode):
                fail(f"special package entry: {path}")
            actual.add(relative)

    expected = set(FILES)
    if actual != expected:
        fail(
            f"{root.name}: inventory mismatch: "
            f"actual={sorted(actual)}, expected={sorted(expected)}"
        )


def parse_sums(data, package):
    text = data.decode("utf-8")
    if not text.endswith("\n"):
        fail(f"{package}: SHA256SUMS must end with LF")

    listed = {}
    previous = None
    for line in text[:-1].split("\n"):
        if "\r" in line:
            fail(f"{package}: SHA256SUMS contains CR")
        fields = line.split("  ", 1)
        if len(fields) != 2:
            fail(f"{package}: malformed SHA256SUMS line: {line!r}")
        digest, relative = fields
        if not re.fullmatch(r"[0-9a-f]{64}", digest) or not safe_relative_path(relative):
            fail(f"{package}: invalid SHA256SUMS entry: {line!r}")
        if relative in ("registration.toml", "SHA256SUMS"):
            fail(f"{package}: metadata listed as payload: {relative}")
        if previous is not None and previous >= relative:
            fail(f"{package}: SHA256SUMS paths are not strictly sorted")
        if relative in listed:
            fail(f"{package}: duplicate SHA256SUMS path: {relative}")
        previous = relative
        listed[relative] = digest

    if set(listed) != set(PAYLOADS):
        fail(f"{package}: SHA256SUMS payload inventory differs")
    if len(listed) != 7:
        fail(f"{package}: expected 7 registered payloads, found {len(listed)}")
    return listed


def verify_package(package, spec):
    root = CORPUS / package
    inventory(root)
    data = {relative: (root / relative).read_bytes() for relative in FILES}

    if sha256(data["SHA256SUMS"]) != spec["sums"]:
        fail(f"{package}: SHA256SUMS digest is not the registered digest")
    if sha256(data["registration.toml"]) != spec["registration"]:
        fail(f"{package}: registration.toml digest is not the registered digest")
    if sha256(data["CASES.json"]) != spec["cases"]:
        fail(f"{package}: CASES.json digest is not the registered digest")

    listed = parse_sums(data["SHA256SUMS"], package)
    for relative, expected in listed.items():
        actual = sha256(data[relative])
        if actual != expected:
            fail(f"{package}: {relative} digest mismatch")

    cases = json.loads(data["CASES.json"])
    if not isinstance(cases, list) or len(cases) != spec["count"]:
        fail(f"{package}: registered case count differs")
    if any(case.get("kind") != spec["kind"] for case in cases):
        fail(f"{package}: registered case kind differs")


def render():
    lines = [
        "// Generated by generate_registered_embedded.py; do not edit by hand.",
        "",
        "pub(crate) static FILES: &[(&str, &[u8])] = &[",
    ]
    for package in PACKAGES:
        for relative in FILES:
            path = f"{package}/{relative}"
            lines.extend(
                [
                    "    (",
                    f'        "{path}",',
                    f'        include_bytes!("../corpus/transclusion/{path}"),',
                    "    ),",
                ]
            )
    lines.extend(["];", ""])
    return "\n".join(lines).encode("utf-8")


def main():
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--check", action="store_true", help="check without writing")
    group.add_argument("--write", action="store_true", help="write the generated Rust file")
    args = parser.parse_args()

    specs = read_specs()
    for package in PACKAGES:
        verify_package(package, specs[package])
    generated = render()

    if args.write:
        OUTPUT.write_bytes(generated)
        return 0

    try:
        current = OUTPUT.read_bytes()
    except FileNotFoundError:
        print(f"generated file is missing: {OUTPUT}", file=sys.stderr)
        return 1
    if current != generated:
        print(f"generated file is stale: {OUTPUT}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, UnicodeError, ValueError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1)
