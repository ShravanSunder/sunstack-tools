"""Author structural config negatives and challenge their discriminating power."""

import copy
from dataclasses import dataclass

from jsonschema import Draft202012Validator
from runner_contracts import JsonObject, JsonValue, as_array, as_object


@dataclass(frozen=True, kw_only=True)
class StructuralConfigCase:
    name: str
    config: JsonObject
    anchor: str | None
    closing_object: bool = False


def structural_config_negatives() -> list[StructuralConfigCase]:
    # Independent C4 literals: no schema or product output generates this table.
    baseline: JsonObject = {
        "rule_sets": {"docs": {"path": "rules/set.json", "files": ["docs/*.md"]}}
    }
    cases = [
        StructuralConfigCase(name="schema-required-rule-sets", config={}, anchor=None),
        StructuralConfigCase(
            name="schema-required-entry-path",
            config={"rule_sets": {"docs": {"files": ["docs/*.md"]}}},
            anchor="docs",
            closing_object=True,
        ),
        StructuralConfigCase(
            name="schema-required-entry-files",
            config={"rule_sets": {"docs": {"path": "rules/set.json"}}},
            anchor="docs",
            closing_object=True,
        ),
    ]
    wrong_types: list[tuple[str, JsonValue]] = [
        ("$schema", 7),
        ("rule_sets", []),
        ("rules", []),
        ("overrides", {}),
        ("ignore_patterns", {}),
        ("backend", []),
        ("limits", []),
    ]
    for field, value in wrong_types:
        config = copy.deepcopy(baseline)
        config[field] = value
        cases.append(
            StructuralConfigCase(
                name="schema-type-" + field.replace("$", ""),
                config=config,
                anchor=field,
            )
        )
    wrong_entries: list[tuple[str, JsonObject, str]] = [
        ("path", {"path": [], "files": ["docs/*.md"]}, "path"),
        ("files", {"path": "rules/set.json", "files": "docs/*.md"}, "files"),
    ]
    for name, entry, anchor in wrong_entries:
        cases.append(
            StructuralConfigCase(
                name="schema-entry-type-" + name,
                config={"rule_sets": {"docs": entry}},
                anchor=anchor,
            )
        )
    invalid_fields: list[tuple[str, str, JsonValue, str | None, bool]] = [
        ("integer-leaf", "limits", {"max_files": "2"}, "max_files", False),
        ("enum-severity", "rules", {"docs/standard": "fatal"}, "rules", True),
        (
            "enum-backend",
            "backend",
            {"kind": "other", "model": "jev-latest"},
            "kind",
            False,
        ),
        ("unknown-root", "unexpected", True, "unexpected", False),
        (
            "required-backend-model",
            "backend",
            {"kind": "jev_openrouter"},
            None,
            True,
        ),
    ]
    for name, field, value, anchor, closing in invalid_fields:
        config = copy.deepcopy(baseline)
        config[field] = value
        cases.append(
            StructuralConfigCase(
                name="schema-" + name,
                config=config,
                anchor=anchor,
                closing_object=closing,
            )
        )
    cases.append(
        StructuralConfigCase(
            name="schema-unknown-entry-field",
            config={
                "rule_sets": {
                    "docs": {
                        "path": "rules/set.json",
                        "files": ["docs/*.md"],
                        "unexpected": True,
                    }
                }
            },
            anchor="unexpected",
        )
    )
    return cases


def authored_location(case: StructuralConfigCase, text: str) -> tuple[int, int, int]:
    lines = text.splitlines()
    if case.anchor is None:
        if case.closing_object:
            return len(lines), 1, len(lines[-1])
        return 1, 1, len(lines[0])
    index, element = next(
        (index, line) for index, line in enumerate(lines) if f'"{case.anchor}"' in line
    )
    if case.closing_object:
        indentation = len(element) - len(element.lstrip())
        index, element = next(
            (position, line)
            for position, line in enumerate(lines[index + 1 :], index + 1)
            if line.lstrip().startswith("}")
            and len(line) - len(line.lstrip()) == indentation
        )
        return index + 1, indentation + 1, len(element)
    return index + 1, element.index(f'"{case.anchor}"') + 1, len(element)


def weakened_schema_sensitivity(
    schema: JsonObject, cases: list[StructuralConfigCase]
) -> dict[str, list[str]]:
    variants = {
        name: copy.deepcopy(schema)
        for name in (
            "required",
            "entry_required",
            "enum",
            "additional_properties",
            "type",
        )
    }
    variants["required"].pop("required", None)
    as_object(as_object(variants["entry_required"]["$defs"])["RuleSetEntry"]).pop(
        "required", None
    )
    enum = as_array(
        as_object(as_object(variants["enum"]["$defs"])["SeveritySetting"])["enum"]
    )
    if "fatal" not in enum:
        enum.append("fatal")
    variants["additional_properties"].pop("additionalProperties", None)
    as_object(as_object(variants["type"]["properties"])["$schema"]).pop("type", None)
    original = Draft202012Validator(schema)
    verdicts = {case.name: original.is_valid(case.config) for case in cases}
    return {
        name: [
            case.name
            for case in cases
            if not verdicts[case.name]
            and Draft202012Validator(variant).is_valid(case.config)
        ]
        for name, variant in variants.items()
    }
