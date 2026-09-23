#!/usr/bin/env python3
"""Translate the registered record-shape inventory into JSON Schema."""

import argparse
import hashlib
import json
from pathlib import Path
import sys


HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
INVENTORY_PATH = ROOT / "xtask" / "tests" / "corpus" / "record-schema" / "INVENTORY.json"
OUTPUT_DIR = ROOT / ".rha" / "schemas"
CONTRACT_PATH = "docs/architecture/record-schema-contract.md"
SCHEMA_URI = "https://json-schema.org/draft/2020-12/schema"
INVENTORY_FORMAT = "rha-observed-shapes-1"

FAMILIES = {
    "acceptance",
    "evidence",
    "h4",
    "h5_conformance",
    "markdown_corpus",
    "policy",
    "task",
}

JSON_TYPES = {
    "null",
    "boolean",
    "integer",
    "number",
    "string",
    "array",
    "object",
}

OBJECT_MODES = {"closed_record", "open_map", "observation_only"}


class InventoryError(ValueError):
    """Raised when the registered inventory is not translatable."""


def fail(message):
    raise InventoryError(message)


def parse_pointer(value, context):
    if not isinstance(value, str):
        fail(f"{context}: JSON Pointer must be a string")
    if value == "":
        return ()
    if not value.startswith("/"):
        fail(f"{context}: invalid JSON Pointer {value!r}")

    parts = []
    for raw in value[1:].split("/"):
        decoded = []
        index = 0
        while index < len(raw):
            character = raw[index]
            if character != "~":
                decoded.append(character)
                index += 1
                continue
            if index + 1 >= len(raw) or raw[index + 1] not in "01":
                fail(f"{context}: invalid JSON Pointer escape in {value!r}")
            decoded.append("~" if raw[index + 1] == "0" else "/")
            index += 2
        parts.append("".join(decoded))
    return tuple(parts)


def pointer(parts):
    return "".join(
        "/" + part.replace("~", "~0").replace("/", "~1") for part in parts
    )


def schema_type(types):
    return types[0] if len(types) == 1 else list(types)


def json_type(value):
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
    fail(f"unsupported JSON value in enum: {type(value).__name__}")


def canonical_json(value):
    return (
        json.dumps(
            value,
            ensure_ascii=False,
            allow_nan=False,
            indent=2,
            sort_keys=True,
        )
        + "\n"
    ).encode("utf-8")


class SchemaBuilder:
    def __init__(self, family, detail):
        self.family = family
        self.detail = detail
        self.nodes = self._node_map(detail.get("nodes"), f"{family}.nodes")
        if () not in self.nodes:
            fail(f"{family}: inventory has no root node")

        self.extensions = self._validate_extensions(detail.get("extensions", []))
        self.extension_by_path = {
            parse_pointer(entry["path"], f"{family}.extensions"): entry
            for entry in self.extensions
        }

        self.selection_variants = []
        self.rules = []
        self._validate_variants(detail.get("variants", []))

    def _node_map(self, value, context):
        if not isinstance(value, dict):
            fail(f"{context}: expected an object")

        result = {}
        for raw_path, node in value.items():
            path = parse_pointer(raw_path, f"{context} path")
            if path in result:
                fail(f"{context}: duplicate decoded path {pointer(path)!r}")
            if not isinstance(node, dict):
                fail(f"{context}{raw_path}: expected an object")
            self._validate_node(node, f"{context}{raw_path}")
            result[path] = node
        return result

    def _validate_node(self, node, context):
        types = node.get("types")
        if (
            not isinstance(types, list)
            or not types
            or any(not isinstance(item, str) for item in types)
            or any(item not in JSON_TYPES for item in types)
            or len(set(types)) != len(types)
        ):
            fail(f"{context}: types must be a non-empty list of unique JSON types")

        if "enforced" in node and not isinstance(node["enforced"], bool):
            fail(f"{context}: enforced must be boolean")

        mode = node.get("object_mode")
        if mode is not None and mode not in OBJECT_MODES:
            fail(f"{context}: unsupported object_mode {mode!r}")
        if mode is not None and "object" not in types:
            fail(f"{context}: object_mode requires object in types")

        if mode == "closed_record":
            for field in ("required_keys", "allowed_keys"):
                value = node.get(field)
                if not isinstance(value, list) or any(
                    not isinstance(item, str) for item in value
                ):
                    fail(f"{context}: {field} must be a list of strings")
            if not set(node["required_keys"]).issubset(set(node["allowed_keys"])):
                fail(f"{context}: required_keys is not contained in allowed_keys")
        elif mode == "open_map":
            if node.get("required_keys") != []:
                fail(f"{context}: open_map required_keys must be []")
            if node.get("allowed_keys") is not None:
                fail(f"{context}: open_map allowed_keys must be null")
            if node.get("map_value_domain") not in ("any", "string"):
                fail(f"{context}: open_map has an unsupported map_value_domain")

    def _validate_extensions(self, extensions):
        if not isinstance(extensions, list):
            fail(f"{self.family}.extensions: expected a list")

        result = []
        seen = set()
        for index, extension in enumerate(extensions):
            context = f"{self.family}.extensions[{index}]"
            if not isinstance(extension, dict):
                fail(f"{context}: expected an object")
            raw_path = extension.get("path")
            path = parse_pointer(raw_path, f"{context}.path")
            if path in seen:
                fail(f"{context}: duplicate path {pointer(path)!r}")
            seen.add(path)

            types = extension.get("types")
            if (
                not isinstance(types, list)
                or not types
                or any(not isinstance(item, str) for item in types)
                or any(item not in JSON_TYPES for item in types)
                or len(set(types)) != len(types)
            ):
                fail(f"{context}: types must be a non-empty list of unique JSON types")

            if "values" in extension and extension["values"] is not None:
                if not isinstance(extension["values"], list):
                    fail(f"{context}.values: expected a list or null")
                for value in extension["values"]:
                    json_type(value)

            if "required_in_parent" in extension:
                required = extension["required_in_parent"]
                if required is not None and not isinstance(required, bool):
                    fail(f"{context}.required_in_parent: expected boolean or null")

            result.append(extension)
        return result

    def _validate_variants(self, variants):
        if not isinstance(variants, list):
            fail(f"{self.family}.variants: expected a list")

        seen = set()
        for index, variant in enumerate(variants):
            context = f"{self.family}.variants[{index}]"
            if not isinstance(variant, dict):
                fail(f"{context}: expected an object")
            path = parse_pointer(variant.get("path"), f"{context}.path")
            if path in seen:
                fail(f"{context}: duplicate path {pointer(path)!r}")
            seen.add(path)
            if path == ():
                fail(f"{context}: a variant cannot replace the root schema")
            if path not in self.nodes:
                fail(f"{context}: path {pointer(path)!r} is absent from pooled nodes")

            if variant.get("rule") == "exactly_one_key_present":
                keys = variant.get("keys")
                if (
                    not isinstance(keys, list)
                    or len(keys) != 2
                    or any(not isinstance(key, str) for key in keys)
                ):
                    fail(f"{context}: exactly_one_key_present requires two keys")
                self.rules.append(variant)
                continue

            selection = variant.get("selection")
            if not isinstance(selection, dict):
                fail(f"{context}: selection is required for a schema variant")
            operator = selection.get("operator")
            if operator == "any_key_present":
                keys = selection.get("keys")
                if (
                    not isinstance(keys, list)
                    or not keys
                    or any(not isinstance(key, str) for key in keys)
                ):
                    fail(f"{context}: any_key_present requires string keys")
            elif operator == "key_present_or_type":
                for field in ("key", "type_key", "type"):
                    if not isinstance(selection.get(field), str):
                        fail(f"{context}: {field} is required for key_present_or_type")
                if selection["type"] not in JSON_TYPES:
                    fail(f"{context}: unsupported selector type")
            else:
                fail(f"{context}: unsupported selector operator {operator!r}")

            for field in ("then", "else"):
                if not isinstance(selection.get(field), str):
                    fail(f"{context}: selection.{field} must be a string")

            branches = variant.get("branches")
            if not isinstance(branches, dict):
                fail(f"{context}: branches must be an object")
            expected = {selection["then"], selection["else"]}
            if set(branches) != expected:
                fail(f"{context}: branches do not match selector branch names")

            normalized_branches = {}
            for branch_name in sorted(expected):
                branch_context = f"{context}.branches[{branch_name!r}]"
                branch = branches[branch_name]
                if not isinstance(branch, dict):
                    fail(f"{branch_context}: expected an object")
                normalized_branches[branch_name] = self._node_map(
                    branch,
                    f"{branch_context}.nodes",
                )
                if path not in normalized_branches[branch_name]:
                    fail(
                        f"{branch_context}: missing branch root {pointer(path)!r}"
                    )

            self.selection_variants.append(
                {
                    "path": path,
                    "selection": selection,
                    "branches": normalized_branches,
                }
            )

    def _extension_at(self, path):
        return self.extension_by_path.get(path)

    def _has_node(self, path, nodes):
        return path in nodes or path in self.extension_by_path

    def _direct_children(self, path, nodes):
        result = {}
        for child in nodes:
            if len(child) != len(path) + 1 or child[:-1] != path:
                continue
            segment = child[-1]
            if segment in ("*", "{}"):
                continue
            if segment in result and result[segment] != child:
                fail(f"{self.family}: duplicate child path {pointer(child)!r}")
            result[segment] = child
        return result

    def _property_names(self, path, node, nodes):
        names = set()
        if node.get("object_mode") == "closed_record":
            names.update(node.get("allowed_keys", []))

        names.update(self._direct_children(path, nodes))
        for extension_path in self.extension_by_path:
            if (
                len(extension_path) == len(path) + 1
                and extension_path[:-1] == path
                and extension_path[-1] not in ("*", "{}")
            ):
                names.add(extension_path[-1])

        for name in names:
            if name in ("*", "{}"):
                fail(
                    f"{self.family}{pointer(path)}: reserved structural key {name!r}"
                )
        return sorted(names)

    def _effective_required(self, path, node):
        required = set(node.get("required_keys", []))
        for extension_path, extension in self.extension_by_path.items():
            if len(extension_path) != len(path) + 1 or extension_path[:-1] != path:
                continue
            setting = extension.get("required_in_parent")
            if setting is True:
                required.add(extension_path[-1])
            elif setting is False:
                required.discard(extension_path[-1])
        return sorted(required)

    def _append_all_of(self, schema, clause):
        schema.setdefault("allOf", []).append(clause)

    def _apply_enum(self, schema, types, extension):
        if extension is None or extension.get("values") is None:
            return
        values = extension["values"]
        self._append_all_of(
            schema,
            {
                "if": {"type": schema_type(types)},
                "then": {"enum": values},
            },
        )

    def _selector_schema(self, selection):
        operator = selection["operator"]
        if operator == "any_key_present":
            return {
                "type": "object",
                "anyOf": [
                    {"required": [key]} for key in selection["keys"]
                ],
            }

        return {
            "type": "object",
            "anyOf": [
                {"required": [selection["key"]]},
                {
                    "required": [selection["type_key"]],
                    "properties": {
                        selection["type_key"]: {"type": selection["type"]},
                    },
                },
            ],
        }

    def _variant_conditionals(self, path, schema, nodes, disabled):
        for variant in self.selection_variants:
            variant_path = variant["path"]
            if len(variant_path) != len(path) + 1 or variant_path[:-1] != path:
                continue
            if variant_path in disabled:
                continue

            property_name = variant_path[-1]
            selection = variant["selection"]
            selector = self._selector_schema(selection)
            then_name = selection["then"]
            else_name = selection["else"]

            then_schema = self._render_node(
                variant_path,
                variant["branches"][then_name],
                disabled | {variant_path},
            )
            else_schema = self._render_node(
                variant_path,
                variant["branches"][else_name],
                disabled | {variant_path},
            )

            self._append_all_of(
                schema,
                {
                    "if": {
                        "required": [property_name],
                        "properties": {property_name: selector},
                    },
                    "then": {"properties": {property_name: then_schema}},
                    "else": {"properties": {property_name: else_schema}},
                },
            )

    def _h4_rule(self, schema, rule):
        first, second = rule["keys"]
        self._append_all_of(
            schema,
            {
                "if": {
                    "type": "object",
                    "not": {
                        "anyOf": [
                            {"required": [first]},
                            {"required": [second]},
                        ],
                    },
                },
                "then": {"required": [first]},
            },
        )
        self._append_all_of(
            schema,
            {
                "if": {
                    "type": "object",
                    "required": [first, second],
                },
                "then": {"not": {}},
            },
        )

    def _render_node(self, path, nodes, disabled):
        if path in {variant["path"] for variant in self.selection_variants}:
            if path not in disabled:
                return {}

        raw_node = nodes.get(path)
        extension = self._extension_at(path)
        if raw_node is None and extension is None:
            fail(f"{self.family}: no inventory node at {pointer(path)!r}")

        types = (
            list(extension["types"])
            if extension is not None
            else list(raw_node["types"])
        )
        mode = raw_node.get("object_mode") if raw_node is not None else None
        enforced = raw_node.get("enforced", True) if raw_node is not None else True
        schema = {"type": schema_type(types)}

        if not enforced:
            self._apply_enum(schema, types, extension)
            return schema

        if mode == "closed_record":
            properties = {}
            for name in self._property_names(path, raw_node, nodes):
                child_path = path + (name,)
                if not self._has_node(child_path, nodes):
                    fail(
                        f"{self.family}{pointer(path)}: allowed key {name!r} "
                        "has no child observation or extension"
                    )
                properties[name] = self._render_node(child_path, nodes, disabled)
            schema["properties"] = properties
            required = self._effective_required(path, raw_node)
            if required:
                schema["required"] = required
            schema["additionalProperties"] = False
        elif mode == "open_map":
            if raw_node["map_value_domain"] == "string":
                schema["additionalProperties"] = {"type": "string"}
            else:
                schema["additionalProperties"] = True
        elif mode == "observation_only":
            pass

        if "array" in types:
            item_path = path + ("*",)
            if raw_node is not None and raw_node.get("item_domain") == "unconstrained_unobserved":
                schema["items"] = {}
            elif self._has_node(item_path, nodes):
                schema["items"] = self._render_node(item_path, nodes, disabled)
            else:
                schema["items"] = {}

        self._apply_enum(schema, types, extension)
        self._variant_conditionals(path, schema, nodes, disabled)
        for rule in self.rules:
            rule_path = parse_pointer(rule["path"], f"{self.family}.variant rule")
            if rule_path == path:
                self._h4_rule(schema, rule)
        return schema

    def render(self):
        return self._render_node((), self.nodes, set())


def validate_inventory(inventory):
    if not isinstance(inventory, dict):
        fail("inventory root must be an object")
    if inventory.get("format") != INVENTORY_FORMAT:
        fail(
            f"inventory format must be {INVENTORY_FORMAT!r}, "
            f"got {inventory.get('format')!r}"
        )
    if not isinstance(inventory.get("source_revision"), str):
        fail("inventory source_revision must be a string")
    families = inventory.get("families")
    if not isinstance(families, dict):
        fail("inventory families must be an object")
    actual = set(families)
    missing = sorted(FAMILIES - actual)
    extra = sorted(actual - FAMILIES)
    if missing:
        fail(f"inventory is missing families: {', '.join(missing)}")
    if extra:
        fail(f"inventory has unsupported families: {', '.join(extra)}")


def title_for(family):
    labels = {
        "acceptance": "acceptance",
        "evidence": "evidence",
        "h4": "H4",
        "h5_conformance": "H5 conformance",
        "markdown_corpus": "Markdown corpus",
        "policy": "policy",
        "task": "task",
    }
    return f"RHA {labels[family]} record schema"


def build_schemas(inventory, inventory_bytes):
    inventory_sha256 = hashlib.sha256(inventory_bytes).hexdigest()
    schemas = {}
    for family in sorted(FAMILIES):
        detail = inventory["families"][family]
        builder = SchemaBuilder(family, detail)
        root_schema = builder.render()
        document = {
            "$schema": SCHEMA_URI,
            "title": title_for(family),
            "description": (
                "Structural record-shape schema translated from the registered "
                "RHA observation inventory; it is not an evaluator."
            ),
            "x-rha-contract": CONTRACT_PATH,
            "x-rha-family": family,
            "x-rha-inventory-format": inventory["format"],
            "x-rha-inventory-sha256": inventory_sha256,
            "x-rha-source-revision": inventory["source_revision"],
        }
        document.update(root_schema)
        schemas[family] = canonical_json(document)
    return schemas


def output_path(family):
    return OUTPUT_DIR / f"{family}.schema.json"


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Generate JSON Schema 2020-12 files from the registered inventory."
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="do not write; fail if any generated schema is missing or differs",
    )
    args = parser.parse_args(argv)

    try:
        inventory_bytes = INVENTORY_PATH.read_bytes()
        try:
            inventory = json.loads(inventory_bytes.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            fail(f"cannot parse {INVENTORY_PATH}: {exc}")
        validate_inventory(inventory)
        schemas = build_schemas(inventory, inventory_bytes)
    except (InventoryError, OSError) as exc:
        print(f"schema_codegen.py: error: {exc}", file=sys.stderr)
        return 2

    if args.check:
        failures = []
        for family in sorted(schemas):
            path = output_path(family)
            if not path.is_file():
                failures.append(f"missing {path}")
                continue
            try:
                actual = path.read_bytes()
            except OSError as exc:
                failures.append(f"cannot read {path}: {exc}")
                continue
            if actual != schemas[family]:
                failures.append(f"different {path}")
        if failures:
            for failure in failures:
                print(f"schema_codegen.py: {failure}", file=sys.stderr)
            return 1
        return 0

    try:
        OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
        for family in sorted(schemas):
            output_path(family).write_bytes(schemas[family])
    except OSError as exc:
        print(f"schema_codegen.py: error: cannot write schemas: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
