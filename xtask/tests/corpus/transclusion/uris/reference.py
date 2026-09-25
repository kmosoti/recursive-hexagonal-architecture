#!/usr/bin/env python3
"""Independent lexical URI-rebasing reference and deterministic corpus generator.

The reference deliberately works on strings and UTF-8 bytes only.  It does not
consult the filesystem, the current directory, a URL parser, or a browser.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import random
import re
import sys
import unicodedata
from pathlib import Path
from typing import Any


SEED = 90092028
GENERATED_CASES = 128
SCHEMA_VERSION = 1
LEGACY_CASE_COUNT = 148
ADDED_CONTROL_COUNT = 5
PAYLOAD_FILES = (
    "CASES.json",
    "README.md",
    "reference.py",
    "selftestreport.txt",
    "source-snapshots/bdr.md",
    "source-snapshots/contract.md",
    "source-snapshots/prompt.md",
)
ALL_PACKAGE_FILES = PAYLOAD_FILES + ("SHA256SUMS", "registration.toml")
README_TEXT = """# Transclusion URI fixture package

This package is a pre-code, independent reference for the bounded lexical URI
rebasing rules in the frozen contract snapshots. It contains 25 hand-reviewed
controls and 128 deterministic cases generated from seed `90092028`. It does
not claim complete URI conformance.

`reference.py` is read-only by default. Its reference operates on canonical
source-derived page IDs, preserves special destinations byte-for-byte, applies
lexical POSIX dot-segment handling, and maps only valid link-only fragments.
Images never use the anchor map. The expected values in `CASES.json` are
checked against the same reference and the controls are explicitly asserted in
the generator source.

## Reproduce from a fresh staging copy

Use a fresh `target/m2` staging directory containing a copy of the committed
eventual package at `xtask/tests/corpus/transclusion/uris`:

```sh
mkdir -p target/m2
cp -R xtask/tests/corpus/transclusion/uris target/m2/transclusion-uris
cd target/m2/transclusion-uris
python3 -B reference.py
python3 -B reference.py --reproduce-to reproduced
python3 -B - <<'PY'
from pathlib import Path

root = Path('.')
copy = root / 'reproduced'
names = sorted(path.relative_to(root) for path in root.rglob('*')
               if path.is_file() and copy not in path.parents)
assert names == sorted(path.relative_to(copy) for path in copy.rglob('*') if path.is_file())
for name in names:
    assert (root / name).read_bytes() == (copy / name).read_bytes()
PY
rm -rf reproduced
```

The first command is a read-only self-test. The second command writes only the
owned `reproduced/` subdirectory of the copied package; the comparison checks
all nine files, not only `CASES.json`. Remove that directory after comparison.
The strict child must be fresh and direct under the package. Do not run
reproduction in the committed package.

The source snapshots are the exact frozen inputs used for this package.
Metadata points at their eventual committed paths and never requires equality
with a future live source file. `SHA256SUMS` covers every payload file except
itself and `registration.toml`, in sorted relative-path order.
"""
_UNRESERVED = frozenset(
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~"
)
_SCHEME_RE = re.compile(r"^[A-Za-z][A-Za-z0-9+.-]*:")
_HEX = frozenset("0123456789abcdefABCDEF")


def _pct_encode(text: str, *, keep_slash: bool = False) -> str:
    """Encode UTF-8 bytes, preserving only RFC-unreserved bytes and optionally /."""

    output: list[str] = []
    for byte in text.encode("utf-8"):
        if byte in _UNRESERVED or (keep_slash and byte == 0x2F):
            output.append(chr(byte))
        else:
            output.append(f"%{byte:02X}")
    return "".join(output)


def encode_page_id(page_id: str) -> str:
    """Encode a canonical source-derived page ID, whose separators are slashes."""

    return _pct_encode(unicodedata.normalize("NFC", page_id), keep_slash=True)


def _decode_fragment(fragment: str) -> str | None:
    """Strictly percent-decode a fragment as UTF-8; + remains a literal plus."""

    data = bytearray()
    index = 0
    while index < len(fragment):
        character = fragment[index]
        if character == "%":
            if index + 2 >= len(fragment):
                return None
            pair = fragment[index + 1 : index + 3]
            if any(digit not in _HEX for digit in pair):
                return None
            data.append(int(pair, 16))
            index += 3
            continue
        data.extend(character.encode("utf-8"))
        index += 1
    try:
        return bytes(data).decode("utf-8", errors="strict")
    except UnicodeDecodeError:
        return None


def _is_scheme_reference(destination: str) -> bool:
    """Apply the contract's ASCII scheme test before any path separator."""

    if destination.startswith("/"):
        return True
    match = _SCHEME_RE.match(destination)
    if match is None:
        return False
    separator = destination.find("/")
    return separator < 0 or match.end() - 1 < separator


def _split_suffix(destination: str) -> tuple[str, str]:
    positions = [position for position in (destination.find("?"), destination.find("#")) if position >= 0]
    if not positions:
        return destination, ""
    split_at = min(positions)
    return destination[:split_at], destination[split_at:]


def _normalize_components(components: list[str]) -> list[str]:
    normalized: list[str] = []
    for component in components:
        if component in ("", "."):
            continue
        if component == "..":
            if normalized and normalized[-1] != "..":
                normalized.pop()
            else:
                normalized.append("..")
            continue
        normalized.append(component)
    return normalized


def _relative_components(target: list[str], base: list[str]) -> list[str]:
    common = 0
    while common < len(target) and common < len(base) and target[common] == base[common]:
        common += 1
    return [".."] * (len(base) - common) + target[common:]


def _relative_path(target: list[str], base: list[str], trailing_slash: bool) -> str:
    components = _relative_components(target, base)
    if not components:
        result = "."
    else:
        result = "/".join(components)
    if trailing_slash:
        result += "/"
    first = result.split("/", 1)[0]
    if _SCHEME_RE.match(first):
        result = "./" + result
    return result


def rebase_uri(
    host: str,
    origin: str,
    destination: str,
    reference_kind: str,
    anchors: dict[str, str],
) -> str:
    """Rebase one ordinary link or image source under the committed contract."""

    if reference_kind not in {"link", "image"}:
        raise ValueError(f"unknown reference kind: {reference_kind}")
    if _is_scheme_reference(destination):
        return destination

    encoded_host = encode_page_id(host)
    encoded_origin = encode_page_id(origin)
    host_directory = encoded_host.split("/")[:-1]
    origin_directory = encoded_origin.split("/")[:-1]
    path, suffix = _split_suffix(destination)

    fragment_only = destination.startswith("#") and path == "" and not destination.startswith("?#")
    if reference_kind == "link" and fragment_only:
        fragment = suffix[1:]
        decoded = _decode_fragment(fragment)
        if decoded is not None and decoded in anchors:
            return "#" + _pct_encode(anchors[decoded])

    if path == "":
        target = origin_directory + [encoded_origin.split("/")[-1] + ".md"]
        trailing_slash = False
    else:
        target = _normalize_components(origin_directory + path.split("/"))
        trailing_slash = path.endswith("/")

    rebased = _relative_path(target, host_directory, trailing_slash)
    return rebased + suffix


def _case(
    case_id: str,
    host: str,
    origin: str,
    destination: str,
    reference_kind: str,
    anchors: dict[str, str],
    expected: str | None = None,
) -> dict[str, Any]:
    return {
        "id": case_id,
        "kind": "uri",
        "host": host,
        "origin": origin,
        "destination": destination,
        "reference_kind": reference_kind,
        "anchors": dict(sorted(anchors.items())),
        "expected": rebase_uri(host, origin, destination, reference_kind, anchors)
        if expected is None
        else expected,
    }


def _legacy_authored_cases() -> list[dict[str, Any]]:
    """The original controls; their rows and expected values remain frozen."""

    controls = [
        ("control-child", "guides/Host", "source/Source", "child.md", "link", {}, "../source/child.md"),
        ("control-parent", "guides/Host", "source/Source", "../peer.md#x", "link", {}, "../peer.md#x"),
        ("control-image", "guides/Host", "source/Source", "asset.png", "image", {}, "../source/asset.png"),
        ("control-matched", "guides/Host", "source/Source", "#inside", "link", {"inside": "mapped-host-anchor"}, "#mapped-host-anchor"),
        ("control-unmatched", "guides/Host", "source/Source", "#outside", "link", {"inside": "mapped-host-anchor"}, "../source/Source.md#outside"),
        ("control-query-fragment", "guides/Host", "source/Source", "?q=1#outside", "link", {"outside": "wrong-kind"}, "../source/Source.md?q=1#outside"),
        ("control-empty", "guides/Host", "source/Source", "", "link", {}, "../source/Source.md"),
        ("control-absolute", "guides/Host", "source/Source", "/assets/icon.svg", "link", {}, "/assets/icon.svg"),
        ("control-scheme", "guides/Host", "source/Source", "https://example.test/a:b", "link", {}, "https://example.test/a:b"),
        ("control-protocol-relative", "guides/Host", "source/Source", "//cdn.example.test/a.png", "image", {}, "//cdn.example.test/a.png"),
        ("control-colon-like", "docs/Host", "docs/Origin", "./a:b.md", "link", {}, "./a:b.md"),
        ("control-directory", "guides/Host", "source/Source", "subdir/", "link", {}, "../source/subdir/"),
        ("control-above-root", "guides/Host", "source/Source", "../../escape.md", "link", {}, "../../escape.md"),
        ("control-unicode-page", "guides/Über", "docs/Élan/Start", "child.md", "link", {}, "../docs/%C3%89lan/child.md"),
        ("control-percent-match", "guides/Host", "source/Source", "#%69nside", "link", {"inside": "mapped-host-anchor"}, "#mapped-host-anchor"),
        ("control-invalid-percent", "guides/Host", "source/Source", "#%ZZ", "link", {"inside": "mapped-host-anchor"}, "../source/Source.md#%ZZ"),
        ("control-plus-literal", "guides/Host", "source/Source", "#a+b", "link", {"a b": "plus-space"}, "../source/Source.md#a+b"),
        ("control-image-anchor", "guides/Host", "source/Source", "#inside", "image", {"inside": "mapped-host-anchor"}, "../source/Source.md#inside"),
        ("control-invalid-utf8", "guides/Host", "source/Source", "#%FF", "link", {"inside": "mapped-host-anchor"}, "../source/Source.md#%FF"),
        ("control-case-sensitive", "guides/Host", "source/Source", "#Inside", "link", {"inside": "mapped-host-anchor"}, "../source/Source.md#Inside"),
    ]
    return [_case(case_id, host, origin, destination, kind, anchors, expected) for case_id, host, origin, destination, kind, anchors, expected in controls]


def _added_authored_cases() -> list[dict[str, Any]]:
    """Clarification controls added after the original 148 rows."""

    controls = [
        (
            "control-supplied-escapes",
            "guides/Host",
            "source/Source",
            "dir%20name/%2E%2E/file.md?x=%2F#f%20g",
            "link",
            {},
            "../source/dir%20name/%2E%2E/file.md?x=%2F#f%20g",
        ),
        (
            "control-unicode-generated-anchor",
            "guides/Host",
            "source/Source",
            "#%C3%BCmlaut",
            "link",
            {"ümlaut": "tx-0-sameünicode"},
            "#tx-0-same%C3%BCnicode",
        ),
        (
            "control-self-import-matched",
            "self/Source",
            "self/Source",
            "#same",
            "link",
            {"same": "tx-0-same"},
            "#tx-0-same",
        ),
        (
            "control-self-import-query",
            "self/Source",
            "self/Source",
            "?view=full",
            "link",
            {"same": "tx-0-same"},
            "Source.md?view=full",
        ),
        (
            "control-self-import-unmatched",
            "self/Source",
            "self/Source",
            "#missing",
            "link",
            {"same": "tx-0-same"},
            "Source.md#missing",
        ),
    ]
    return [_case(case_id, host, origin, destination, kind, anchors, expected) for case_id, host, origin, destination, kind, anchors, expected in controls]


def authored_cases() -> list[dict[str, Any]]:
    return _legacy_authored_cases() + _added_authored_cases()


def _seeded_case(rng: random.Random, number: int) -> dict[str, Any]:
    hosts = [
        "guides/Host",
        "docs/Reference",
        "Host",
        "manual/Part-1/Host",
        "unicode/Überblick",
        "a/b/C",
    ]
    origins = [
        "source/Source",
        "source/peer",
        "Origin",
        "manual/Part-2/Entry",
        "unicode/Élan/Start",
        "a/b/Other",
        "deep/one/two/Three",
    ]
    destinations = [
        "child.md",
        "../peer.md#x",
        "./same.md",
        "../../escape.md",
        "assets/image.png",
        "nested/",
        "?q=1",
        "?q=1#frag",
        "#inside",
        "#outside",
        "#%69nside",
        "#a+b",
        "#%ZZ",
        "#%FF",
        "a:b.md",
        "./a:b.md",
        "/absolute/path",
        "mailto:user@example.test",
        "//cdn.example.test/a.png",
        "ümlaut/child.md",
    ]
    host = rng.choice(hosts)
    origin = rng.choice(origins)
    destination = rng.choice(destinations)
    reference_kind = rng.choice(["link", "image"])
    anchors = rng.choice(
        [
            {},
            {"inside": "mapped-inside"},
            {"outside": "mapped-outside", "inside": "mapped-inside"},
            {"a+b": "plus-anchor", "a b": "space-anchor"},
            {"ümlaut": "mapped-ümlaut"},
        ]
    )
    return _case(f"seeded-{number:03d}", host, origin, destination, reference_kind, anchors)


def build_cases() -> list[dict[str, Any]]:
    rng = random.Random(SEED)
    cases = _legacy_authored_cases()
    cases.extend(_seeded_case(rng, number) for number in range(1, GENERATED_CASES + 1))
    cases.extend(_added_authored_cases())
    return cases


def cases_bytes(cases: list[dict[str, Any]]) -> bytes:
    return (json.dumps(cases, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode("utf-8")


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _selftest_lines(case_count: int) -> list[str]:
    return [
        f"PASS cases={case_count}",
        f"PASS legacy_rows_preserved={LEGACY_CASE_COUNT}",
        f"PASS authored_controls={len(authored_cases())}",
        f"PASS added_controls={ADDED_CONTROL_COUNT}",
        f"PASS seeded_cases={GENERATED_CASES} seed={SEED}",
        "PASS differential expected-values=reference for every row",
        "PASS control coverage=empty/query/fragment/scheme/path/image/unicode/percent/dot-segments/NFC/self-import",
        "PASS full-package metadata=derived from frozen snapshots",
        "PASS read-only mode=no files written",
    ]


def _selftest_report_bytes(case_count: int) -> bytes:
    return ("\n".join(_selftest_lines(case_count)) + "\n").encode("utf-8")


def _sha256sums_bytes(payloads: dict[str, bytes]) -> bytes:
    lines = [f"{_sha256(payloads[name])}  {name}" for name in sorted(payloads)]
    return ("\n".join(lines) + "\n").encode("utf-8")


def _registration_bytes(payloads: dict[str, bytes], sums: bytes, case_count: int) -> bytes:
    return f"""schema_version = {SCHEMA_VERSION}
eventual_package = "xtask/tests/corpus/transclusion/uris"
seed = {SEED}
case_count = {case_count}
kind_counts = {{ uri = {case_count} }}
payload_count = {len(payloads)}
content_sha256 = "{_sha256(sums)}"
model = "gpt-5.6-luna"
effort = "high"

[source_digests]
contract_sha256 = "{_sha256(payloads['source-snapshots/contract.md'])}"
bdr_sha256 = "{_sha256(payloads['source-snapshots/bdr.md'])}"
prompt_sha256 = "{_sha256(payloads['source-snapshots/prompt.md'])}"

[source_paths]
contract = "xtask/tests/corpus/transclusion/uris/source-snapshots/contract.md"
bdr = "xtask/tests/corpus/transclusion/uris/source-snapshots/bdr.md"
prompt = "xtask/tests/corpus/transclusion/uris/source-snapshots/prompt.md"

[original_paths]
contract = "docs/architecture/transclusion-contract.md"
bdr = "docs/adr/BDR-0007-site-owns-page-tree.md"
prompt = "xtask/tests/corpus/write-prompts/CHG-008/pd-transclusion-uris-generator-prompt.md"
""".encode("utf-8")


def _package_files(package: Path) -> dict[str, bytes]:
    cases = build_cases()
    payloads = {
        "CASES.json": cases_bytes(cases),
        "README.md": README_TEXT.encode("utf-8"),
        "reference.py": (package / "reference.py").read_bytes(),
        "selftestreport.txt": _selftest_report_bytes(len(cases)),
        "source-snapshots/bdr.md": (package / "source-snapshots/bdr.md").read_bytes(),
        "source-snapshots/contract.md": (package / "source-snapshots/contract.md").read_bytes(),
        "source-snapshots/prompt.md": (package / "source-snapshots/prompt.md").read_bytes(),
    }
    sums = _sha256sums_bytes(payloads)
    return {
        **payloads,
        "SHA256SUMS": sums,
        "registration.toml": _registration_bytes(payloads, sums, len(cases)),
    }


def _validate_case(row: dict[str, Any]) -> None:
    required = {"id", "kind", "host", "origin", "destination", "reference_kind", "anchors", "expected"}
    if set(row) != required:
        raise AssertionError(f"{row.get('id', '<unknown>')}: fields differ")
    if row["kind"] != "uri":
        raise AssertionError(f"{row['id']}: wrong kind")
    if row["reference_kind"] not in {"link", "image"}:
        raise AssertionError(f"{row['id']}: wrong reference kind")
    if not isinstance(row["anchors"], dict):
        raise AssertionError(f"{row['id']}: anchors is not an object")
    actual = rebase_uri(row["host"], row["origin"], row["destination"], row["reference_kind"], row["anchors"])
    if actual != row["expected"]:
        raise AssertionError(f"{row['id']}: expected {row['expected']!r}, reference returned {actual!r}")


def _self_test(package: Path) -> list[str]:
    cases_path = package / "CASES.json"
    cases = json.loads(cases_path.read_text(encoding="utf-8"))
    if not isinstance(cases, list):
        raise AssertionError("CASES.json is not an array")
    for row in cases:
        _validate_case(row)
    if len(cases) != LEGACY_CASE_COUNT + ADDED_CONTROL_COUNT:
        raise AssertionError("case count is not legacy rows plus clarification controls")
    if cases[:LEGACY_CASE_COUNT] != build_cases()[:LEGACY_CASE_COUNT]:
        raise AssertionError("one of the original 148 rows changed")
    if [row["id"] for row in cases[-ADDED_CONTROL_COUNT:]] != [
        row["id"] for row in _added_authored_cases()
    ]:
        raise AssertionError("clarification controls are not last and ordered")
    generated = cases_bytes(build_cases())
    if generated != cases_path.read_bytes():
        raise AssertionError("CASES.json is not the deterministic generator output")
    expected_files = _package_files(package)
    actual_files = {
        str(path.relative_to(package))
        for path in package.rglob("*")
        if path.is_file()
    }
    if actual_files != set(ALL_PACKAGE_FILES):
        raise AssertionError(f"package file set differs: {sorted(actual_files)}")
    for name, expected in expected_files.items():
        actual = (package / name).read_bytes()
        if actual != expected:
            raise AssertionError(f"{name} is not deterministic from frozen inputs")
    return _selftest_lines(len(cases))


def _reproduce(package: Path, target: str) -> Path:
    destination = Path(target)
    if not destination.is_absolute():
        destination = package / destination
    destination = destination.resolve()
    package_root = package.resolve()
    try:
        destination.relative_to(package_root)
    except ValueError as error:
        raise ValueError("--reproduce-to must be inside the reference.py package directory") from error
    if destination == package_root or destination.parent != package_root:
        raise ValueError("--reproduce-to must name a direct child directory")
    if destination.exists():
        raise ValueError("--reproduce-to must name a fresh child directory")
    destination.mkdir()
    for name, content in _package_files(package).items():
        output = destination / name
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(content)
    return destination


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reproduce-to", metavar="DIR", help="write deterministic CASES.json below this package")
    args = parser.parse_args(argv)
    package = Path(__file__).resolve().parent
    try:
        if args.reproduce_to:
            destination = _reproduce(package, args.reproduce_to)
            print(f"reproduced {len(ALL_PACKAGE_FILES)} package files under {destination.name}/")
            return 0
        for line in _self_test(package):
            print(line)
        return 0
    except (AssertionError, OSError, ValueError, json.JSONDecodeError) as error:
        print(f"FAIL {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
