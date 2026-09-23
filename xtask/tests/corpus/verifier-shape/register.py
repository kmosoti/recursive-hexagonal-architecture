#!/usr/bin/env python3
"""Parse and hash a registration; never evaluate records or verifier inputs."""
import argparse
import collections
import hashlib
import json
from pathlib import Path
import tomllib

HERE = Path(__file__).resolve().parent
EXCLUDED = {"registration.toml", "GENERATOR-PROMPT.md"}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def checked_files():
    rows = []
    for path in sorted(p for p in HERE.rglob("*") if p.is_file()):
        assert not path.is_symlink(), path
        data = path.read_bytes()
        if path.suffix == ".json":
            json.loads(data)
        elif path.suffix == ".toml":
            tomllib.loads(data.decode())
        relative = path.relative_to(HERE).as_posix()
        if relative not in EXCLUDED:
            rows.append((relative, sha(data), len(data)))
    return sorted(rows, key=lambda row: row[0])


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--check", action="store_true", help="Compare the computed registration bytes; do not write.")
    args = ap.parse_args()
    kind = HERE.name
    assert kind in ("record-schema", "verifier-shape")
    index_path = HERE / ("SOURCE-INDEX.json" if kind == "record-schema" else "metadata/SOURCE-INDEX.json")
    index = json.loads(index_path.read_bytes())
    for row in index["sources"]:
        data = (HERE / row["snapshot"]).read_bytes()
        assert sha(data) == row["sha256"] and len(data) == row["bytes"], row["path"]
    source_lines = "".join(f'{row["path"]} {row["sha256"]}\n' for row in sorted(index["sources"], key=lambda x: x["path"]))
    assert index["source_tree_sha256"] == sha(source_lines.encode())
    assert index["source_count"] == len(index["sources"])
    basis = json.loads((HERE / "metadata/BASIS-INDEX.json").read_bytes())
    history = json.loads((HERE / "metadata/GENERATION-HISTORY.json").read_bytes())
    for row in basis["inputs"]:
        data = (HERE / row["snapshot"]).read_bytes()
        assert sha(data) == row["sha256"] and len(data) == row["bytes"], row["source_path"]
    rows = checked_files()
    tree = sha("".join(f"{path} {digest}\n" for path, digest, size in rows).encode())
    data = {
        "schema_version": 1,
        "registered_in": "CHG-019.1 (P-C stage 0)",
        "registration": kind,
        "generation": "sampled",
        "sample": 1,
        "interrupted_pre_payload_attempts": history["interrupted_pre_payload_attempts"],
        "failed_deterministic_self_checks": history["failed_deterministic_self_checks"],
        "interrupted_deterministic_self_checks": history["interrupted_deterministic_self_checks"],
        "failed_inspection_commands": history["failed_inspection_commands"],
        "incomplete_section_extractions": history["incomplete_section_extractions"],
        "generation_history_sha256": sha((HERE / "metadata/GENERATION-HISTORY.json").read_bytes()),
        "attempt_note": "Same independent session and sample 1. One pre-payload SIGINT; two failed inspection commands and two incomplete section extractions retained in history. Completed deterministic self-checks passed. Wire labels clarified before registration/commit or production grading. No random generator or seed was used.",
        "source_revision": index["source_revision"],
        "source_count": index["source_count"],
        "source_tree_sha256": index["source_tree_sha256"],
        "source_index_sha256": sha(index_path.read_bytes()),
        "basis_file_count": len(basis["inputs"]),
        "basis_index_sha256": sha((HERE / "metadata/BASIS-INDEX.json").read_bytes()),
        "prompt_path": "GENERATOR-PROMPT.md",
        "prompt_sha256": sha((HERE / "GENERATOR-PROMPT.md").read_bytes()),
        "correction_prompt_path": "GENERATOR-CORRECTION.md",
        "correction_prompt_sha256": sha((HERE / "GENERATOR-CORRECTION.md").read_bytes()),
        "wire_clarification_prompt_path": "GENERATOR-WIRE-CLARIFICATION.md",
        "wire_clarification_prompt_sha256": sha((HERE / "GENERATOR-WIRE-CLARIFICATION.md").read_bytes()),
        "record_contract_sha256": sha((HERE / "inputs/record-schema-contract.md").read_bytes()),
        "verifier_contract_sha256": sha((HERE / "inputs/verifier-contract.md").read_bytes()),
        "tree_sha256": tree,
        "payload_file_count": len(rows),
        "payload_bytes": sum(size for _, _, size in rows),
        "tree_algorithm": "sha256 of sorted UTF-8 '<relative path> <sha256>\\n' lines; exclude only registration.toml and GENERATOR-PROMPT.md; include README, correction and wire-clarification prompts",
        "validation": "JSON/TOML parsing, hashes and deterministic derivation/construction only; no production schema/lint/verifier execution",
    }
    family_counts = None
    if kind == "record-schema":
        corpus = json.loads((HERE / "CASES.json").read_bytes())
        cases = corpus["cases"]
        assert len({c["id"] for c in cases}) == len(cases)
        data.update(case_count=len(cases), baseline_count=len(corpus["baselines"]),
                    unmodified_source_controls=sum(c["id"].startswith("RS-control-source-") for c in cases),
                    structurally_clean_case_count=sum(not c["expected_schema_codes"] for c in cases),
                    structurally_rejected_case_count=sum(bool(c["expected_schema_codes"]) for c in cases),
                    supported_source_count=sum(row["family"] is not None for row in index["sources"]),
                    excluded_source_count=sum(row["family"] is None for row in index["sources"]),
                    inventory_sha256=sha((HERE / "INVENTORY.json").read_bytes()),
                    cases_sha256=sha((HERE / "CASES.json").read_bytes()))
        family_counts = collections.Counter(c["family"] for c in cases)
    else:
        fixtures = [json.loads(p.read_bytes()) for p in sorted(HERE.glob("S*.json"))]
        assert [f["id"] for f in fixtures] == [f"S{i:03}" for i in range(1, len(fixtures) + 1)]
        assert set(HERE.glob("*.json")) == set(HERE.glob("S*.json"))
        data.update(case_count=len(fixtures), malformed_case_count=sum("malformed" in f["expected"] for f in fixtures),
                    well_formed_case_count=sum("malformed" not in f["expected"] for f in fixtures),
                    recipes_sha256=sha((HERE / "metadata/RECIPES.json").read_bytes()),
                    diagnostic_spelling="Published grouped labels in verifier-contract §4.1; precise offending JSON Pointer retained unchanged in RECIPES and CASE-INDEX.")
    lines = ["# Independent pre-implementation registration. No schema or evaluator generated."]
    lines += [f"{key} = {json.dumps(value, ensure_ascii=False)}" for key, value in data.items()]
    lines += ['generator = { model = "gpt-6-astra", reasoning_effort = "xhigh" }']
    if family_counts:
        lines += ["", "[case_counts_by_family]"]
        lines += [f"{key} = {value}" for key, value in sorted(family_counts.items())]
        lines += ["", "[source_counts_by_family]"]
        lines += [f"{key} = {value}" for key, value in sorted(index["family_counts"].items())]
    content = ("\n".join(lines) + "\n").encode()
    tomllib.loads(content.decode())
    target = HERE / "registration.toml"
    if args.check:
        assert target.read_bytes() == content, "registration differs from payload; do not silently re-register after evaluation"
    else:
        target.write_bytes(content)
    print(json.dumps({k: data[k] for k in ("registration", "case_count", "source_count", "payload_file_count", "tree_sha256", "prompt_sha256", "correction_prompt_sha256", "wire_clarification_prompt_sha256")}, sort_keys=True))


if __name__ == "__main__":
    main()
