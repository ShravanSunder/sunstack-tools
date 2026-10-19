"""Require the actual authored inventory to detect weakened schema axes."""

import json

import pytest
from jsonschema import Draft202012Validator
from runner_contracts import JsonObject
from schema_fixtures import (
    authored_location,
    structural_config_negatives,
    weakened_schema_sensitivity,
)


def literal_config_schema() -> JsonObject:
    return {
        "type": "object",
        "required": ["rule_sets"],
        "additionalProperties": False,
        "properties": {
            "$schema": {"type": ["string", "null"]},
            "rule_sets": {
                "type": "object",
                "minProperties": 1,
                "additionalProperties": {"$ref": "#/$defs/RuleSetEntry"},
            },
            "rules": {
                "type": "object",
                "additionalProperties": {"$ref": "#/$defs/SeveritySetting"},
            },
            "overrides": {"type": "array"},
            "ignore_patterns": {"type": "array", "items": {"type": "string"}},
            "limits": {
                "type": "object",
                "properties": {"max_files": {"type": "integer", "minimum": 1}},
            },
            "backend": {
                "type": "object",
                "required": ["kind", "model"],
                "properties": {
                    "kind": {"const": "jev_openrouter"},
                    "model": {"type": "string"},
                },
                "additionalProperties": False,
            },
        },
        "$defs": {
            "RuleSetEntry": {
                "type": "object",
                "required": ["path", "files"],
                "properties": {
                    "path": {"type": "string"},
                    "files": {"type": "array", "items": {"type": "string"}},
                },
                "additionalProperties": False,
            },
            "SeveritySetting": {"type": "string", "enum": ["error", "warn", "off"]},
        },
    }


@pytest.mark.parametrize(
    "constraint",
    ["required", "entry_required", "enum", "additional_properties", "type"],
)
def test_each_weakened_schema_changes_a_verdict_in_the_real_inventory(
    constraint: str,
) -> None:
    changed = weakened_schema_sensitivity(
        literal_config_schema(), structural_config_negatives()
    )
    assert changed[constraint], (
        f"Authored inventory cannot detect weakened {constraint}"
    )


def test_independent_structural_negatives_are_invalid_and_have_authored_location_spans() -> (
    None
):
    cases = structural_config_negatives()
    assert len({case.name for case in cases}) == len(cases)
    validator = Draft202012Validator(literal_config_schema())
    for case in cases:
        assert not validator.is_valid(case.config), case.name
        text = json.dumps(case.config, indent=2) + "\n"
        line, first, last = authored_location(case, text)
        assert 1 <= line <= len(text.splitlines())
        assert 1 <= first <= last <= len(text.splitlines()[line - 1])
