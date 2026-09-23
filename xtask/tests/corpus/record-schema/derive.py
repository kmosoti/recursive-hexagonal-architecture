#!/usr/bin/env python3
"""Derive observations, never a schema or a validator. Python 3.11+, stdlib only."""
import argparse
import collections
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib

REVISION = "99a74b042842b4d869ef341f58fdfcafb9c96e84"
HERE = Path(__file__).resolve().parent
OUTCOMES = ["passed", "failed", "not_run", "not_applicable", "inconclusive"]
FULL_KEYS = ["harness", "task_scope", "permissions", "budgets", "reasoning_effort"]
# Only these exact, observed semantic paths are exceptions to record closure.
# A '*' segment denotes array pooling, NOT a wildcard over property names.
MAPS = {
    "evidence": {
        "/observed_checks/*/params": ("any", "Check parameter names are semantic keys."),
        "/observed_checks/*/selection_counts": ("any", "Check-kind measurement names form a nullable count dictionary."),
        "/evidence_inputs/environment/ci": ("any", "Environment variable names are input keys."),
    },
    "policy": {"/strictness": ("any", "Strictness parameter names are semantic keys.")},
    "h4": {
        "/fixtures/inputs_sha256": ("string", "Input path to digest."),
        "/fixtures/generated_inputs_sha256": ("string", "Generated input path to digest."),
        "/cases/*/fixture_inputs_sha256": ("string", "Historic per-case input path to digest."),
        "/cases/*/report": ("any", "Embedded checker report: separately owned vocabulary."),
        "/cases/*/witness": ("any", "Checker witness payload: separately owned vocabulary."),
        "/cases/*/grade/false_alarms/*": ("any", "Embedded checker/compiler finding payload."),
        "/cases/*/grade/registered_facts/*": ("any", "Embedded checker finding payload."),
    },
}


def canonical(value):
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False,
                       allow_nan=False) + "\n").encode()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def pointer(path):
    return "".join("/" + p.replace("~", "~0").replace("/", "~1") for p in path)


def json_type(value):
    if value is None:
        return "null"
    return {bool: "boolean", int: "integer", float: "number", str: "string",
            list: "array", dict: "object"}[type(value)]


def to_json(value):
    if isinstance(value, (datetime.datetime, datetime.date, datetime.time)):
        return value.isoformat()
    if isinstance(value, dict):
        return {k: to_json(v) for k, v in value.items()}
    if isinstance(value, list):
        return [to_json(v) for v in value]
    return value


def parse(path, data):
    return to_json(tomllib.loads(data.decode())) if path.endswith(".toml") else json.loads(data)


def family(path, value):
    if path.startswith(".rha/tasks/"):
        return "task"
    if path.startswith(".rha/acceptances/"):
        return "acceptance"
    if path == ".rha/policy.toml":
        return "policy"
    if value.get("record_kind") == "evidence":
        return "evidence"
    if value.get("kind") in ("h4", "markdown_corpus", "h5_conformance"):
        return value["kind"]
    return None


def git(*args):
    # Read-only object operations, scoped to the pinned paths/revision.
    return subprocess.check_output(["git", *args])


def sources(from_git):
    if from_git:
        paths = git("ls-tree", "-r", "--name-only", REVISION, "--",
                    ".rha/tasks", ".rha/acceptances", ".rha/policy.toml", "evidence").decode().splitlines()
        paths = [p for p in paths if
                 (p.startswith("evidence/") and p.endswith(".json")) or
                 (p.startswith((".rha/tasks/", ".rha/acceptances/")) and p.endswith(".toml") and p.count("/") == 2) or
                 p == ".rha/policy.toml"]
        return [(p, git("show", f"{REVISION}:{p}")) for p in sorted(paths)]
    index = json.loads((HERE / "SOURCE-INDEX.json").read_bytes())
    rows = []
    for row in index["sources"]:
        data = (HERE / row["snapshot"]).read_bytes()
        assert sha(data) == row["sha256"], row["path"]
        rows.append((row["path"], data))
    return rows


def observe(rows, fam, prefix=()):
    """Pool observations; never decide whether an input is valid."""
    acc = {}

    def visit(value, parts, source, opaque=False):
        p = pointer(parts)
        n = acc.setdefault(p, {"types": collections.Counter(), "sources": set(),
                               "objects": [], "array_lengths": [], "enforced": not opaque})
        n["types"][json_type(value)] += 1
        n["sources"].add(source)
        boundary = MAPS.get(fam, {}).get(p)
        if isinstance(value, dict):
            n["objects"].append(set(value))
            if boundary:
                n["map"] = boundary
            for key, child in sorted(value.items()):
                # Opaque payload objects retain documentary field paths; digest,
                # parameter and environment maps pool their keyed values at '{}'.
                payload = boundary and ("payload" in boundary[1] or "report" in boundary[1])
                segment = key if not boundary or payload else "{}"
                visit(child, parts + (segment,), source, opaque or bool(boundary))
        elif isinstance(value, list):
            n["array_lengths"].append(len(value))
            for child in value:
                visit(child, parts + ("*",), source, opaque)

    for source, value in rows:
        visit(value, prefix, source)
    result = {}
    for p, a in sorted(acc.items()):
        n = {"types": sorted(a["types"]), "type_observations": dict(sorted(a["types"].items())),
             "source_count": len(a["sources"]), "source_paths": sorted(a["sources"]),
             "enforced": a["enforced"]}
        if a["objects"]:
            required = sorted(set.intersection(*a["objects"]))
            allowed = sorted(set.union(*a["objects"]))
            n.update(object_observations=len(a["objects"]),
                     empty_object_observations=sum(not ks for ks in a["objects"]),
                     observed_required_keys=required, observed_allowed_keys=allowed)
            if "map" in a:
                value_type, reason = a["map"]
                n.update(object_mode="open_map", required_keys=[], allowed_keys=None,
                         map_value_domain=value_type, map_reason=reason)
            else:
                n.update(object_mode="closed_record" if a["enforced"] else "observation_only",
                         required_keys=required, allowed_keys=allowed)
        if a["array_lengths"]:
            n.update(array_observations=len(a["array_lengths"]),
                     empty_array_observations=a["array_lengths"].count(0),
                     item_observations=sum(a["array_lengths"]),
                     item_domain="observed" if p + "/*" in acc else "unconstrained_unobserved")
        result[p] = n
    return result


def inventory(rows):
    grouped = collections.defaultdict(list)
    index = []
    for p, data in rows:
        value = parse(p, data)
        fam = family(p, value)
        index.append({"path": p, "snapshot": "sources/" + p, "sha256": sha(data),
                      "bytes": len(data), "family": fam,
                      "role": "derivation" if fam else "excluded_unsupported_evidence_payload"})
        if fam:
            grouped[fam].append((p, value))
    result = {"format": "rha-observed-shapes-1", "source_revision": REVISION,
              "outcomes": OUTCOMES, "families": {}, "dynamic_maps": [],
              "diagnostics": {"missing": "schema.missing_field", "type": "schema.wrong_type",
                              "unknown_key": "schema.unknown_field", "unsupported_value": "schema.invalid_value"}}
    for fam, records in sorted(grouped.items()):
        nodes = observe(records, fam)
        variants = []
        extensions = []
        if fam == "evidence":
            agent = [(p, v["agent_context"]) for p, v in records]
            sourced = [(p, v) for p, v in agent if "source" in v or "provenance" in v]
            legacy = [(p, v) for p, v in agent if "source" not in v and "provenance" not in v]
            variants.append({"path": "/agent_context", "selection": {"operator": "any_key_present", "keys": ["source", "provenance"], "then": "sourced", "else": "legacy"},
                             "branches": {"sourced": observe(sourced, fam, ("agent_context",)), "legacy": observe(legacy, fam, ("agent_context",))},
                             "branch_observations": {"sourced": len(sourced), "legacy": len(legacy)}})
            prov = [(p, v["provenance"]) for p, v in sourced]
            full = [(p, v) for p, v in prov if any(k in v for k in FULL_KEYS)]
            compact = [(p, v) for p, v in prov if not any(k in v for k in FULL_KEYS)]
            branches = {"full": observe(full, fam, ("agent_context", "provenance")),
                        "compact": observe(compact, fam, ("agent_context", "provenance"))}
            assert set(branches["full"]["/agent_context/provenance"]["required_keys"]) == set(FULL_KEYS)
            assert branches["compact"]["/agent_context/provenance"]["required_keys"] == ["instruction_sources", "model_id"]
            variants.append({"path": "/agent_context/provenance", "active_in": {"/agent_context": "sourced"},
                             "selection": {"operator": "any_key_present", "keys": FULL_KEYS, "then": "full", "else": "compact"},
                             "branches": branches, "branch_observations": {"full": len(full), "compact": len(compact)}})
            claims = [(p, v["change_claim"]) for p, v in records]
            sourced_claim = [(p, v) for p, v in claims if "intent" in v or isinstance(v.get("source"), dict)]
            null_claim = [(p, v) for p, v in claims if "intent" not in v and not isinstance(v.get("source"), dict)]
            variants.append({"path": "/change_claim", "selection": {"operator": "key_present_or_type", "key": "intent", "type_key": "source", "type": "object", "then": "sourced", "else": "unsourced"},
                             "branches": {"sourced": observe(sourced_claim, fam, ("change_claim",)), "unsourced": observe(null_claim, fam, ("change_claim",))},
                             "branch_observations": {"sourced": len(sourced_claim), "unsourced": len(null_claim)}})
            extensions += [{"path": "/observed_checks/*/outcome", "required_in_parent": True, "types": ["string"], "values": OUTCOMES},
                           {"path": "/observed_checks/*/policy_rule_id", "required_in_parent": False, "types": ["string"]},
                           {"path": "/observed_checks/*/rationale", "required_in_parent": False, "types": ["string"]}]
        if fam in ("h4", "markdown_corpus", "h5_conformance"):
            extensions.append({"path": "/summary/outcome", "types": ["string"], "values": OUTCOMES})
        if fam == "h4":
            cases = [c for _, v in records for c in v["cases"]]
            variants.append({"path": "/cases/*", "rule": "exactly_one_key_present", "keys": ["fixture_path", "fixture_inputs_sha256"],
                             "neither_reason": "schema.missing_field", "both_reason": "schema.invalid_value",
                             "observations": {"fixture_path_only": sum("fixture_path" in c and "fixture_inputs_sha256" not in c for c in cases),
                                              "fixture_inputs_sha256_only": sum("fixture_inputs_sha256" in c and "fixture_path" not in c for c in cases),
                                              "both": sum("fixture_path" in c and "fixture_inputs_sha256" in c for c in cases),
                                              "neither": sum("fixture_path" not in c and "fixture_inputs_sha256" not in c for c in cases)}})
        versions = sorted({v["schema_version"] for _, v in records})
        extensions.append({"path": "/schema_version", "types": ["integer"], "values": versions})
        discriminator = {"evidence": {"record_kind": "evidence"}, "h4": {"kind": "h4"},
                         "markdown_corpus": {"kind": "markdown_corpus"}, "h5_conformance": {"kind": "h5_conformance"}}.get(fam)
        if discriminator:
            for k, v in discriminator.items():
                extensions.append({"path": "/" + k, "types": ["string"], "values": [v], "required_in_parent": True})
        result["families"][fam] = {"record_count": len(records), "source_paths": [p for p, _ in records],
                                    "observed_schema_versions": versions, "nodes": nodes,
                                    "variants": variants, "extensions": extensions,
                                    "identification": discriminator or {"required_keys": {"task": ["id", "mode"], "acceptance": ["acceptor"], "policy": ["authority", "lanes"]}[fam]}}
        for p, (domain, reason) in sorted(MAPS.get(fam, {}).items()):
            assert p in nodes and "object" in nodes[p]["types"], (fam, p)
            result["dynamic_maps"].append({"family": fam, "path": p, "types": nodes[p]["types"], "values": domain, "reason": reason})
    counts = collections.Counter(x["family"] or "excluded" for x in index)
    source_index = {"source_revision": REVISION, "source_count": len(index), "family_counts": dict(sorted(counts.items())),
                    "source_tree_sha256": sha("".join(f'{x["path"]} {x["sha256"]}\n' for x in index).encode()), "sources": index}
    result["source_tree_sha256"] = source_index["source_tree_sha256"]
    return source_index, result


def emit(path, data, check):
    if check:
        assert path.read_bytes() == data, f"nonreproducible: {path}"
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--from-git", action="store_true", help="Read only git ls-tree/show at the pinned revision; otherwise use hashed snapshots.")
    ap.add_argument("--check", action="store_true", help="Compare bytes without writes; this checks derivation, NOT schema validity.")
    args = ap.parse_args()
    rows = sources(args.from_git)
    idx, inv = inventory(rows)
    for path, data in rows:
        emit(HERE / "sources" / path, data, args.check)
    emit(HERE / "SOURCE-INDEX.json", canonical(idx), args.check)
    emit(HERE / "INVENTORY.json", canonical(inv), args.check)
    unobserved = []
    for fam, detail in inv["families"].items():
        for path, node in detail["nodes"].items():
            if node["enforced"] and node.get("item_domain") == "unconstrained_unobserved":
                unobserved.append({"family": fam, "path": path, "scope": "pooled", "array_observations": node["array_observations"]})
        for variant in detail["variants"]:
            for branch, nodes in variant.get("branches", {}).items():
                for path, node in nodes.items():
                    if node["enforced"] and node.get("item_domain") == "unconstrained_unobserved":
                        unobserved.append({"family": fam, "path": path, "scope": variant["path"] + ":" + branch,
                                           "array_observations": node["array_observations"]})
    emit(HERE / "metadata" / "UNOBSERVED-ARRAYS.json", canonical(unobserved), args.check)
    print(json.dumps({"source_count": idx["source_count"], "family_counts": idx["family_counts"],
                      "source_tree_sha256": idx["source_tree_sha256"], "inventory_sha256": sha(canonical(inv))}, sort_keys=True))


if __name__ == "__main__":
    main()
