#!/usr/bin/env python3
"""Write registered construction recipes; no schema validation or semantic grading."""
import argparse
import copy
import json
from pathlib import Path
import sys
sys.dont_write_bytecode = True
from derive import HERE, FULL_KEYS, OUTCOMES, canonical, emit, parse, pointer


def apply_mutations(value, mutations):
    """Strict JSON Pointer remove/set; set may add only the final object key."""
    value = copy.deepcopy(value)
    for mutation in mutations:
        path = mutation["path"]
        if path == "":
            assert mutation["op"] == "set", "root removal is not defined"
            value = copy.deepcopy(mutation["value"])
            continue
        assert path.startswith("/")
        parts = [p.replace("~1", "/").replace("~0", "~") for p in path[1:].split("/")]
        parent = value
        for part in parts[:-1]:
            parent = parent[int(part)] if isinstance(parent, list) else parent[part]
        key = int(parts[-1]) if isinstance(parent, list) else parts[-1]
        if mutation["op"] == "remove":
            del parent[key]
        else:
            assert mutation["op"] == "set"
            if isinstance(parent, list):
                assert 0 <= key < len(parent), "array set replaces, never appends"
            parent[key] = copy.deepcopy(mutation["value"])
    return value


def locate(value, semantic_path, wanted_type=None):
    parts = [s.replace("~1", "/").replace("~0", "~") for s in semantic_path[1:].split("/")] if semantic_path else []
    def visit(v, i, concrete):
        if i == len(parts):
            if wanted_type is None or isinstance(v, wanted_type):
                return pointer(concrete), v
            return None
        part = parts[i]
        if part == "*":
            if isinstance(v, list):
                for j, item in enumerate(v):
                    found = visit(item, i + 1, concrete + (str(j),))
                    if found is not None:
                        return found
        elif isinstance(v, dict) and part in v:
            return visit(v[part], i + 1, concrete + (part,))
        return None
    return visit(value, 0, ())


def build():
    inventory = json.loads((HERE / "INVENTORY.json").read_bytes())
    index = json.loads((HERE / "SOURCE-INDEX.json").read_bytes())
    baselines, values, by_family = {}, {}, {}
    for row in index["sources"]:
        fam = row["family"]
        if fam is None:
            continue
        name = row["path"]
        baselines[name] = {"family": fam, "source": row["snapshot"], "source_sha256": row["sha256"], "mutations": []}
        values[name] = parse(name, (HERE / row["snapshot"]).read_bytes())
        by_family.setdefault(fam, []).append(name)
    cases = []

    def add(name, baseline, mutations, codes, purpose):
        cases.append({"id": name, "family": baselines[baseline]["family"], "baseline": baseline,
                      "mutations": mutations, "expected_schema_codes": sorted(set(codes)), "purpose": purpose})
        # This is ONLY pointer resolution/JSON serialization, never an evaluator.
        canonical(apply_mutations(values[baseline], mutations))

    def setting(path, value):
        return {"op": "set", "path": path, "value": value}

    def removing(path):
        return {"op": "remove", "path": path}

    missing = ["schema.missing_field"]
    wrong = ["schema.wrong_type"]
    unknown = ["schema.unknown_field"]
    invalid = ["schema.invalid_value"]
    for i, name in enumerate(sorted(baselines), 1):
        add(f"RS-control-source-{i:03}", name, [], [], "Unmodified frozen structural control; no semantic acceptance claim: " + name)

    def witness(fam, path, wanted_type=None):
        for name in by_family[fam]:
            found = locate(values[name], path, wanted_type)
            if found is not None:
                return name, *found
        raise AssertionError((fam, path, wanted_type))

    seq = 0
    def generated(label, baseline, mutations, codes, purpose):
        nonlocal seq
        seq += 1
        add(f"RS-{seq:04}-{label}", baseline, mutations, codes, purpose)

    for fam, detail in sorted(inventory["families"].items()):
        for path, node in sorted(detail["nodes"].items()):
            if not node["enforced"]:
                continue
            # Branch-specific observations replace these pooled subtrees below.
            if fam == "evidence" and any(path == p or path.startswith(p + "/") for p in ("/agent_context", "/change_claim")):
                continue
            name, concrete, value = witness(fam, path)
            types = node["types"]
            bad = next(v for typ, v in [("boolean", True), ("array", []), ("string", "bad-type"), ("object", {}), ("null", None)] if typ not in types)
            generated("wrong-type", name, [setting(concrete, bad)], wrong,
                      f"{fam} {path or '<root>'}: replace the observed type domain {types} with an excluded type.")
            if node.get("object_mode") == "closed_record":
                name, concrete, obj = witness(fam, path, dict)
                for key in node["required_keys"]:
                    generated("missing-root" if not path else "missing-nested", name,
                              [removing(concrete + pointer((key,)))], missing,
                              f"{fam} {path or '<root>'}: delete required record key {key}.")
                generated("unknown-key", name, [setting(concrete + "/__unregistered_field__", "new")], unknown,
                          f"{fam} {path or '<root>'}: an unregistered nested/root record key is closed.")
            if node.get("item_domain") == "unconstrained_unobserved":
                generated("unobserved-array-control", name, [setting(concrete, [17, {"unobserved": True}])], [],
                          f"{fam} {path}: all frozen arrays were empty; no item-domain constraint may be guessed.")

        name = by_family[fam][0]
        add("RS-version-unsupported-" + fam, name, [setting("/schema_version", 999)], invalid,
            "Version is outside the exact observed family version set.")
        if fam in ("evidence", "h4", "markdown_corpus", "h5_conformance"):
            key = "record_kind" if fam == "evidence" else "kind"
            add("RS-discriminator-unsupported-" + fam, name, [setting("/" + key, "unregistered_report")], invalid,
                "Declared family still selects the structural shape; unsupported discriminator is invalid_value.")
        if fam in ("h4", "markdown_corpus", "h5_conformance"):
            for state in OUTCOMES:
                add(f"RS-summary-outcome-{fam}-{state}", name, [setting("/summary/outcome", state)], [],
                    "Five normative states are structural outcomes; no rerun or semantic acceptance is asserted.")
            add("RS-summary-outcome-unknown-" + fam, name, [setting("/summary/outcome", "green")], invalid,
                "Unsupported corpus summary outcome.")
            # Every summary count and every case scalar is already covered by the
            # generic type cases. These names make case-array regressions easy to find.
            add("RS-case-item-type-" + fam, name, [setting("/cases/0", "not-a-case")], wrong,
                "Corpus cases are typed record items, never an arbitrary map or an L0 check list.")

    ev = by_family["evidence"]
    full = next(n for n in reversed(ev) if "source" in values[n]["agent_context"] and any(k in values[n]["agent_context"]["provenance"] for k in FULL_KEYS))
    compact = next(n for n in ev if "source" in values[n]["agent_context"] and not any(k in values[n]["agent_context"]["provenance"] for k in FULL_KEYS))
    legacy = next(n for n in reversed(ev) if "source" not in values[n]["agent_context"])
    branch_baselines = {"/agent_context": {"legacy": legacy, "sourced": full},
                        "/agent_context/provenance": {"full": full, "compact": compact},
                        "/change_claim": {"sourced": full, "unsourced": legacy}}
    # Register selected-branch requiredness and member types without attempting
    # evaluation of any recipe. Selectors are deliberately retained by deletion.
    for variant in inventory["families"]["evidence"]["variants"]:
        root = variant["path"]
        for branch, nodes in sorted(variant["branches"].items()):
            base = branch_baselines[root][branch]
            for path, node in sorted(nodes.items()):
                if root == "/agent_context" and path.startswith("/agent_context/provenance"):
                    continue
                if not node["enforced"]:
                    continue
                found = locate(values[base], path, dict if "object" in node["types"] else None)
                if found is None:
                    # Optional documentary fields may exist in another record of
                    # this same branch. Find their source via observed source paths.
                    for candidate in node["source_paths"]:
                        found = locate(values[candidate], path, dict if "object" in node["types"] else None)
                        if found is not None:
                            base = candidate
                            break
                assert found is not None, (root, branch, path)
                concrete, val = found
                if path != root:
                    bad = next(v for typ, v in [("boolean", True), ("array", []), ("string", "bad-type"), ("object", {}), ("null", None)] if typ not in node["types"])
                    # Mutating a selector can intentionally select another branch;
                    # those cases are assigned explicit expectations below instead.
                    if not (root == "/change_claim" and branch == "unsourced" and path == "/change_claim/source"):
                        generated("variant-type", base, [setting(concrete, bad)], wrong,
                                  f"Selected {root} {branch}: wrong member type at {path}.")
                if node.get("object_mode") == "closed_record":
                    for key in node["required_keys"]:
                        # All branches keep at least one selector after these
                        # deletions. Unsourced source deletion does not select full.
                        generated("variant-missing", base, [removing(concrete + pointer((key,)))], missing,
                                  f"Selected {root} {branch}: required {path}/{key} is absent.")
                    generated("variant-unknown", base, [setting(concrete + "/__unregistered_field__", True)], unknown,
                              f"Selected {root} {branch}: nested record is closed.")

    add("RS-agent-context-empty", full, [setting("/agent_context", {})], missing,
        "Empty agent_context selects legacy and lacks its four required fields.")
    for member in ("agent_context", "change_claim"):
        add("RS-variant-container-type-" + member, full, [setting("/" + member, False)], wrong,
            "A mistyped variant container gives only wrong_type before child requirements are considered.")
    add("RS-variant-container-type-provenance", full, [setting("/agent_context/provenance", False)], wrong,
        "A mistyped provenance container gives only wrong_type before branch member requirements.")
    add("RS-agent-context-mixed", full, [setting("/agent_context/harness", "legacy value")], unknown,
        "source/provenance selects sourced; direct legacy field is forbidden.")
    add("RS-agent-context-partial-hybrid", legacy,
        [setting("/agent_context/source", values[full]["agent_context"]["source"])], missing + unknown,
        "source selects sourced; provenance is absent and the four legacy keys are forbidden.")
    add("RS-provenance-empty", full, [setting("/agent_context/provenance", {})], missing,
        "Empty provenance selects compact and lacks both model_id and instruction_sources.")
    add("RS-provenance-mixed-compact-full", compact,
        [setting("/agent_context/provenance/harness", "new full selector")], missing + wrong,
        "harness selects full; remaining full required keys are missing and compact string items violate the full object-item instruction_sources domain.")
    full_prov = values[full]["agent_context"]["provenance"]
    add("RS-provenance-full-string-instructions", full,
        [setting("/agent_context/provenance/instruction_sources", ["AGENTS.md"])], wrong,
        "Full provenance retains object-array instruction citations.")
    add("RS-provenance-compact-object-instructions", compact,
        [setting("/agent_context/provenance/instruction_sources", [{"path": "AGENTS.md", "sha256": "0" * 64}])], wrong,
        "Compact provenance retains string-array instruction citations.")
    add("RS-provenance-full-model-id-control", full,
        [setting("/agent_context/provenance/model_id", "future-model")], [],
        "model_id is also allowed in full provenance; its presence is not itself a hybrid.")
    add("RS-claim-empty", legacy, [setting("/change_claim", {})], missing,
        "Empty claim selects unsourced, whose task/source/note keys remain required.")
    add("RS-claim-sourced-hybrid-note", full, [setting("/change_claim/note", "unsourced note")], unknown,
        "Sourced claim permits its full intent fields, not the unsourced note.")
    add("RS-claim-unsourced-hybrid-field", legacy, [setting("/change_claim/non_goals", [])], unknown,
        "Without intent or object source the null branch remains selected; non_goals is forbidden.")
    add("RS-claim-null-task-in-full", full, [setting("/change_claim/task", None)], wrong,
        "Full claim retains a string task despite the other branch's null task.")
    add("RS-claim-null-source-in-full", full, [setting("/change_claim/source", None)], wrong,
        "intent keeps the full branch selected; source cannot be null there.")
    for state in OUTCOMES:
        add("RS-evidence-outcome-" + state, full, [setting("/observed_checks/0/outcome", state)], [],
            "Structural outcome domain only; this mutation may intentionally violate semantic outcome rules.")
    add("RS-not-applicable-extension-control", full,
        [setting("/observed_checks/0/outcome", "not_applicable"), setting("/observed_checks/0/policy_rule_id", "root.documented"), setting("/observed_checks/0/rationale", "registered structural control")], [],
        "Optional policy_rule_id and rationale string extensions are explicitly admitted.")
    for key in ("policy_rule_id", "rationale"):
        add("RS-extension-type-" + key, full, [setting("/observed_checks/0/" + key, 12)], wrong,
            "Explicit optional extension remains string typed.")

    # Make a citation-free copy by recipe, so CLI regressions have no pre-existing
    # file-digest mismatch. The exact eight passed check entries are retained.
    safe = "cli-safe-l0"
    setup = [setting("/verification_identity/configuration", []), setting("/verification_identity/instruction_sources", []),
             setting("/evidence_inputs/fixtures", []), setting("/artifact_identity/untracked_inputs", [])]
    setup += [setting(f"/observed_checks/{i}/artifacts", []) for i in range(len(values[legacy]["observed_checks"]))]
    assert len(values[legacy]["observed_checks"]) == 8
    assert all(c["outcome"] == "passed" and c["exit_status"] == 0 for c in values[legacy]["observed_checks"])
    baselines[safe] = {**baselines[legacy], "mutations": setup, "purpose": "Semantic CLI baseline: current passed CI record, all path/sha256 citation arrays empty; synthetic structural control, not real evidence."}
    values[safe] = apply_mutations(values[legacy], setup)
    add("RS-cli-safe-l0-control", safe, [], [], "Clean baseline for later CLI regressions; no production lint was run.")
    add("RS-cli-missing-outcome", safe, [removing("/observed_checks/0/outcome")], missing,
        "Semantically isolated CLI regression: missing outcome must not be silently accepted.")
    add("RS-cli-unknown-outcome", safe, [setting("/observed_checks/0/outcome", "green")], invalid,
        "Semantically isolated CLI regression: unknown outcome must not be silently accepted.")
    h5 = next(n for n in by_family["h5_conformance"] if isinstance(values[n]["corpus"], dict))
    add("RS-cli-supported-report-control", h5, [], [], "H5 report has its own family and is not treated as eight-check L0 evidence.")
    add("RS-cli-unsupported-report-kind", h5, [setting("/kind", "h5_conformance_unregistered")], invalid,
        "Unsupported structured report kind is rejected; it cannot fall through to accepted empty data.")

    h4path = next(n for n in by_family["h4"] if "fixture_path" in values[n]["cases"][0])
    h4map = next(n for n in by_family["h4"] if "fixture_inputs_sha256" in values[n]["cases"][0])
    for name, base, identity in [("path", h4path, "fixture_path"), ("map", h4map, "fixture_inputs_sha256")]:
        add("RS-h4-identity-neither-" + name, base, [removing("/cases/0/" + identity)], missing,
            "H4 case requires exactly one fixture identity; neither is missing_field.")
        other = "fixture_inputs_sha256" if identity == "fixture_path" else "fixture_path"
        add("RS-h4-identity-both-" + name, base, [setting("/cases/0/" + other, {} if other.endswith("sha256") else "fixture/path")], invalid,
            "Both identity keys violate the H4 XOR; this is invalid_value, not unknown_field.")
    add("RS-h4-documentary-outcome-control", h4path,
        [setting("/held_out/outcome", "not_part_of_this_run")], [],
        "Documentary exclusion status is a string, outside the normative five check/summary states.")
    add("RS-h4-module-level-control", h4path, [setting("/level", "module")], [],
        "H4 level supports module as well as crate; one historical ordinary string is not an enum.")
    add("RS-policy-negative-cooling-off-structural-control", by_family["policy"][0],
        [setting("/authority/cooling_off_hours", -1)], [],
        "Record-schema contract derives integer type but defines no minimum; verifier fixture u64 constraints do not transfer to this family.")

    for boundary in inventory["dynamic_maps"]:
        fam, path = boundary["family"], boundary["path"]
        name, concrete, _ = witness(fam, path, dict)
        generated("map-empty-control", name, [setting(concrete, {})], [], "Dynamic map has no incidental required keys: " + path)
        generated("map-new-key-control", name, [setting(concrete + "/new~1semantic~0key", "0" * 64 if boundary["values"] == "string" else {"arbitrary": [True, None, 1]})], [],
                  "Dynamic map accepts an unseen key; opaque payload descendants are not frozen: " + path)
        if boundary["values"] == "string":
            generated("digest-map-value-type", name, [setting(concrete + "/new~1path", 23)], wrong,
                      "Every path-to-digest map value must remain a string: " + path)
    # Ordinary strings have no inferred enums or semantic regexes.
    add("RS-ordinary-task-mode-string-control", by_family["task"][0], [setting("/mode", "future-mode")], [],
        "Only structural type is derived for ordinary TOML task mode; its observed values do not create an enum.")
    add("RS-no-revision-regex-control", full, [setting("/artifact_identity/revision", "not-a-revision")], [],
        "Revision semantic defects do not alter the structural schema.* projection.")
    add("RS-no-digest-regex-control", full, [setting("/verification_identity/policy_digest", "not-a-digest")], [],
        "Digest semantic defects do not alter the structural schema.* projection.")
    add("RS-structural-reasons-accumulate", safe,
        [removing("/finished_at"), setting("/schema_version", 999), setting("/lane", []), setting("/__unregistered_field__", True)],
        missing + invalid + wrong + unknown,
        "Independent structural failures accumulate as exactly four deduplicated code categories.")
    ids = [c["id"] for c in cases]
    assert len(ids) == len(set(ids))
    return {"format": "rha-structural-recipes-1", "grading": "Exact deduplicated structural schema.* code set only; no semantic acceptance assertion.",
            "inventory": "INVENTORY.json", "baselines": baselines, "cases": cases}


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--materialize", metavar="CASE_ID", help="Write one materialized JSON value to stdout; no validation.")
    args = ap.parse_args()
    result = build()
    if args.materialize:
        case = next(c for c in result["cases"] if c["id"] == args.materialize)
        baseline = result["baselines"][case["baseline"]]
        value = parse(baseline["source"], (HERE / baseline["source"]).read_bytes())
        value = apply_mutations(value, baseline["mutations"])
        sys.stdout.buffer.write(canonical(apply_mutations(value, case["mutations"])))
    else:
        emit(HERE / "CASES.json", canonical(result), args.check)
        print(json.dumps({"case_count": len(result["cases"]), "baseline_count": len(result["baselines"])}))


if __name__ == "__main__":
    main()
