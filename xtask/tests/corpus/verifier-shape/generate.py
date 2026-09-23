#!/usr/bin/env python3
"""Construct supplementary verifier fixtures from frozen V data and §3.

No production or reference evaluator is imported or invoked. Expected predicates
are copied from registered baselines or assigned explicitly by the contract.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
REVISION = "99a74b042842b4d869ef341f58fdfcafb9c96e84"
IDS = ["V015", "V031", "V127", "V129", "V130", "V147", "V148"]
PREFIX = "tools/rha-verifier/tests/corpus/"


def canonical(value):
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False, allow_nan=False) + "\n").encode()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def emit(path, data, check):
    if check:
        assert path.read_bytes() == data, f"nonreproducible: {path}"
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def change(value, mutation):
    parts = mutation["path"][1:].split("/")
    parts = [p.replace("~1", "/").replace("~0", "~") for p in parts]
    obj = value
    for part in parts[:-1]:
        obj = obj[int(part)] if isinstance(obj, list) else obj[part]
    key = int(parts[-1]) if isinstance(obj, list) else parts[-1]
    if mutation["op"] == "remove":
        del obj[key]
    else:
        obj[key] = copy.deepcopy(mutation["value"])


def wire_label(pointer):
    """Published verifier-contract §4.1 labels, separate from precise pointers."""
    parts = pointer[1:].split("/")
    if parts == ["policy"]:
        return "policy.digest"  # First checked policy member in §4.1 order.
    if parts == ["evidence"]:
        return "evidence.producer"  # First checked evidence member.
    if parts[0] in ("local_policies", "surface"):
        return parts[0]
    if parts[0] in ("policy", "evidence"):
        return ".".join(parts[:2])
    if parts[0] == "exception" and len(parts) > 1:
        groups = {
            "exception.subject, base, policy, issuer, issued_at, expires_at":
                ("subject", "base", "policy", "issuer", "issued_at", "expires_at"),
            "exception.authentic, revoked": ("authentic", "revoked"),
            "exception.waived": ("waived",),
            "exception.reason, compensating_control, follow_up":
                ("reason", "compensating_control", "follow_up"),
        }
        return next(label for label, members in groups.items() if parts[1] in members)
    assert len(parts) == 1, pointer
    return parts[0]


def build(originals):
    fixtures, recipes = [], []

    def add(base, purpose, mutation=None, malformed_member=None, updates=None):
        value = copy.deepcopy(originals[base])
        mutations = [] if mutation is None else [mutation]
        for m in mutations:
            change(value, m)
        id = f"S{len(fixtures) + 1:03}"
        value["id"] = id
        value["row"] = "CHG-019.1 verifier-contract §3"
        value["purpose"] = purpose
        expected = copy.deepcopy(originals[base]["expected"])
        if malformed_member:
            expected = {"r_eff": None, "conflict": False, "authentic": False,
                        "applicable": False, "complete": False, "passed": False,
                        "eligible": False, "valid_exception": None if value.get("exception") is None else False,
                        "merge_allowed": False, "malformed": wire_label(malformed_member)}
        if updates:
            expected.update(updates)
        value["expected"] = expected
        recipes.append({"id": id, "baseline": base, "mutations": mutations,
                        "malformed_member": malformed_member, "purpose": purpose,
                        "expectation_derivation": "§3 malformed refusal" if malformed_member else
                        "Original registered expected object" + (" with explicit §1.2/§1.3/§3 changes: " + json.dumps(updates, sort_keys=True) if updates else " unchanged")})
        fixtures.append(value)

    def set_value(base, path, value, purpose, malformed=True, updates=None):
        add(base, purpose, {"op": "set", "path": path, "value": value}, path if malformed else None, updates)

    def remove(base, path, purpose, malformed=True, updates=None):
        add(base, purpose, {"op": "remove", "path": path}, path if malformed else None, updates)

    def required(base, path, bad):
        remove(base, path, "Missing required member " + path + ".")
        set_value(base, path, bad, "Wrong type for required member " + path + ".")

    for base, label in [("V127", "Valid null-exception test control"), ("V148", "Valid independent object-exception control, no logged_at required"),
                        ("V147", "Valid Solo object-exception control with logged_at"), ("V015", "Valid local check/rule and comparison control"),
                        ("V129", "Valid performed comparison control"), ("V130", "Valid matching corpus digest control"),
                        ("V031", "Original negative selected_tests is well formed and fails Passed")]:
        add(base, label + "; retain the original expected object exactly.")

    required("V127", "/policy", [])
    for field in ("digest", "profile"):
        required("V127", "/policy/" + field, 3)
    for field in ("checks", "rules", "default_obligations", "non_waivable", "trusted_producers", "required_inputs", "exception_authority", "acceptance_authority"):
        required("V127", "/policy/" + field, {})
        if field not in ("checks", "rules"):
            set_value("V127", "/policy/" + field + "/0", 7, "Policy string-array item has wrong type: " + field + ".")
    required("V127", "/policy/cooling_off_hours", "24")
    for label, number in [("negative", -1), ("fractional", 0.5), ("u64 overflow", 2 ** 64), ("boolean", True), ("null", None)]:
        set_value("V127", "/policy/cooling_off_hours", number, "Cooling-off must be an unsigned JSON integer representable as u64: " + label + ".")
    for number in (0, 2 ** 64 - 1):
        set_value("V127", "/policy/cooling_off_hours", number, "Valid u64 cooling-off boundary; null exception makes the delay irrelevant.", False)

    def checks_rules(base, prefix):
        set_value(base, prefix + "/checks/0", "not-an-object", "Each check must be an object.")
        for field, bad in (("id", 3), ("kind", False), ("params", [])):
            required(base, prefix + "/checks/0/" + field, bad)
        set_value(base, prefix + "/rules/0", "not-an-object", "Each rule must be an object.")
        for field, bad in (("id", 3), ("scope", False), ("requires", {})):
            required(base, prefix + "/rules/0/" + field, bad)
        set_value(base, prefix + "/rules/0/requires/0", None, "Rule requires must contain only strings.")
    checks_rules("V127", "/policy")

    required("V015", "/local_policies", {})
    set_value("V015", "/local_policies/0", False, "A local policy item must be an object.")
    required("V015", "/local_policies/0/scope", [])
    required("V015", "/local_policies/0/checks", {})
    required("V015", "/local_policies/0/rules", {})
    checks_rules("V015", "/local_policies/0")

    for field in ("base", "candidate_tree", "acceptor", "accountable_change_authority", "now"):
        required("V127", "/" + field, 12)
    required("V127", "/surface", "crates/core/src/lib.rs")
    set_value("V127", "/surface", [], "Surface must be non-empty, even though an empty array has no non-string items.")
    set_value("V127", "/surface/0", 7, "Surface item must be a string.")
    # §3 does not impose non-empty string contents. An empty path changes rule
    # triggering, so that semantic interaction is outside these shape fixtures.

    required("V127", "/evidence", [])
    for field in ("producer", "integrity", "subject", "policy", "base"):
        required("V127", "/evidence/" + field, 12)
    required("V127", "/evidence/inputs", [])
    required("V127", "/evidence/entries", {})
    set_value("V127", "/evidence/entries/0", None, "An evidence entry must be an object.")
    for field, bad in (("id", 3), ("kind", 3), ("params", []), ("outcome", 3)):
        required("V127", "/evidence/entries/0/" + field, bad)
    for bad in ("12", True, None, [], {}):
        set_value("V127", "/evidence/entries/0/selected_tests", bad, "If present, selected_tests must be a JSON number; booleans are not numbers.")
    for bad in (1, "true", None):
        set_value("V129", "/evidence/entries/0/performed", bad, "If present, performed must be boolean.")
    for bad in (1, [], None):
        set_value("V130", "/evidence/entries/0/corpus_digest", bad, "If present, corpus_digest must be string.")

    fail_passed = {"passed": False, "eligible": False, "merge_allowed": False}
    for label, number in [("negative integer", -1), ("negative fraction", -0.5), ("positive fraction", 0.5), ("fraction above one", 1.5),
                          ("zero", 0), ("u64 overflow", 2 ** 64)]:
        set_value("V127", "/evidence/entries/0/selected_tests", number,
                  "Well-formed numeric selected_tests, " + label + "; fails whole-positive-u64 validity, no malformed member.", False, fail_passed)
    for number in (1, 12, 2 ** 64 - 1):
        set_value("V127", "/evidence/entries/0/selected_tests", number,
                  "Positive whole u64 selected_tests; all registered baseline predicates stay true.", False)
    for base, member in (("V127", "selected_tests"), ("V129", "performed"), ("V130", "corpus_digest")):
        remove(base, "/evidence/entries/0/" + member, "Kind-specific member is optional for shape, but its absence fails Passed.", False, fail_passed)
    for state in ("failed", "not_run", "not_applicable", "inconclusive", "unknown-outcome"):
        set_value("V127", "/evidence/entries/0/outcome", state,
                  "String outcome is well formed in verifier fixture §3 but does not satisfy Passed.", False, fail_passed)

    remove("V127", "/exception", "The exception key is required; absence is not an explicit null.")
    for bad in (False, "none", []):
        set_value("V127", "/exception", bad, "Exception must be null or object; supplied non-null malformed exception gives valid_exception=false.")
    strings = ("subject", "base", "policy", "issuer", "issued_at", "expires_at", "reason", "compensating_control", "follow_up")
    for field in strings:
        required("V148", "/exception/" + field, 7)
    for field in ("authentic", "revoked"):
        required("V148", "/exception/" + field, "true")
    required("V148", "/exception/waived", {})
    set_value("V148", "/exception/waived/0", 7, "Exception waived must contain strings.")
    invalid_exception = {"valid_exception": False, "merge_allowed": False}
    for field in ("reason", "compensating_control", "follow_up"):
        set_value("V148", "/exception/" + field, "", "Empty required exception content is shape-valid but invalidates the exception.", False, invalid_exception)
    remove("V147", "/exception/logged_at", "logged_at is shape-optional; missing timestamp prevents a self-issued Solo exception.", False, invalid_exception)
    for bad in ("not-a-timestamp", 7, None, {}):
        set_value("V147", "/exception/logged_at", bad, "logged_at cannot parse on the Solo self-issued path; invalid exception, not malformed fixture.", False, invalid_exception)
    set_value("V148", "/exception/logged_at", {"unused": True},
              "Independent issuer does not use logged_at; §3 supplies no unconditional type rule for that optional member.", False)
    return fixtures, recipes


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--from-git", action="store_true")
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    index, originals = [], {}
    for id in IDS:
        path = PREFIX + id + ".json"
        local = HERE / "sources" / (id + ".json")
        data = subprocess.check_output(["git", "show", REVISION + ":" + path]) if args.from_git else local.read_bytes()
        emit(local, data, args.check)
        originals[id] = json.loads(data)
        index.append({"id": id, "path": path, "snapshot": "sources/" + id + ".json", "sha256": sha(data), "bytes": len(data)})
    fixtures, recipes = build(originals)
    for fixture in fixtures:
        emit(HERE / (fixture["id"] + ".json"), canonical(fixture), args.check)
    emit(HERE / "metadata" / "SOURCE-INDEX.json", canonical({"source_revision": REVISION, "source_count": len(index), "sources": index,
         "source_tree_sha256": sha("".join(f'{x["path"]} {x["sha256"]}\n' for x in index).encode())}), args.check)
    emit(HERE / "metadata" / "RECIPES.json", canonical({"format": "rha-verifier-shape-recipes-1", "cases": recipes}), args.check)
    lines = ["# Supplementary fixture index", "", "| ID | V baseline | Mutation | Offending JSON Pointer | expected.malformed / well-formed |", "| --- | --- | --- | --- | --- |"]
    for recipe in recipes:
        mutation = recipe["mutations"][0] if recipe["mutations"] else None
        description = "unchanged control" if mutation is None else mutation["op"] + " " + mutation["path"]
        if mutation and "value" in mutation:
            description += " = " + json.dumps(mutation["value"], sort_keys=True)
        lines.append(f'| {recipe["id"]} | {recipe["baseline"]} | `{description}` | `{recipe["malformed_member"] or "none"}` | `{wire_label(recipe["malformed_member"]) if recipe["malformed_member"] else "well formed"}` |')
    emit(HERE / "metadata" / "CASE-INDEX.md", ("\n".join(lines) + "\n").encode(), args.check)
    print(json.dumps({"case_count": len(fixtures), "malformed_count": sum("malformed" in x["expected"] for x in fixtures),
                      "well_formed_count": sum("malformed" not in x["expected"] for x in fixtures), "source_count": len(index)}))


if __name__ == "__main__":
    main()
