"""Challenge schema agreement without substituting product behavior."""

import pytest
from jsonschema import Draft202012Validator
from schema_agreement import (
    compare_acceptance,
    config_check_expectation,
    config_instance,
    reject_external_references,
)


def test_unknown_sibling_cannot_pass_agreement_through_an_overpermissive_schema() -> (
    None
):
    instance = config_instance('{"rule_sets":{"docs":{}},"unknown":true}')
    loose = Draft202012Validator({"type": "object"})
    strict = Draft202012Validator(
        {
            "type": "object",
            "properties": {"rule_sets": {"type": "object"}},
            "additionalProperties": False,
        }
    )
    assert not compare_acceptance("structural", False, 2, loose.is_valid(instance), 2)
    assert compare_acceptance("structural", False, 2, strict.is_valid(instance), 2)
    assert not compare_acceptance("semantic", True, 2, True, 0)


def test_duplicate_names_remain_parser_refusals_instead_of_last_value_winning() -> None:
    assert config_instance('{"rules":{"docs/one":"off"}}') == {
        "rules": {"docs/one": "off"}
    }
    for raw in ('{"rules":{},"rules":{}}', '{"rule_sets":{"docs":{},"docs":{}}}'):
        with pytest.raises(ValueError, match="Duplicate"):
            config_instance(raw)


def test_offline_schema_check_refuses_remote_references_but_accepts_draft_metadata() -> (
    None
):
    reject_external_references(
        {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$ref": "#/$defs/Local",
            "$defs": {"Local": {"type": "object"}},
        }
    )
    for keyword in ("$ref", "$dynamicRef"):
        with pytest.raises(ValueError, match="local references"):
            reject_external_references(
                {"properties": {"x": {keyword: "https://example.invalid/remote"}}}
            )


def test_glob_grammar_is_binary_only_while_numeric_ranges_remain_structural() -> None:
    category, schema_accepts, binary_exit = config_check_expectation("bad-glob")
    assert (category, schema_accepts, binary_exit) == ("semantic", True, 2)
    assert compare_acceptance(category, schema_accepts, binary_exit, True, 2)
    assert not compare_acceptance(category, schema_accepts, binary_exit, True, 0)
    assert config_check_expectation("unknown-field") == ("structural", False, 2)
    for name in ("limit-u32-overflow", "limit-u64-overflow"):
        assert config_check_expectation(name) == ("structural", False, 2)


def test_integral_decimal_spelling_is_refused_only_by_the_binary() -> None:
    schema = Draft202012Validator(
        {"type": "integer", "minimum": 1, "maximum": 4294967295}
    )
    assert schema.is_valid(config_instance('{"max_files":1.0}')["max_files"])
    category, accepted, exit_code = config_check_expectation("limit-integral-decimal")
    assert (category, accepted, exit_code) == ("semantic", True, 2)
    assert compare_acceptance(category, accepted, exit_code, True, 2)
    assert not compare_acceptance(category, accepted, exit_code, True, 0)
