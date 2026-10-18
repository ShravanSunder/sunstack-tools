"""Reject false-green observations without invoking or changing the product."""

import copy
import json

import pytest
from assemble_and_print_rule_sources import QUESTION_FILES, require_questions  # pyright: ignore[reportImplicitRelativeImport]
from check_rule_refusals import (  # pyright: ignore[reportImplicitRelativeImport]
    MARKER_GUIDANCE,
    require_located_refusal,
    require_marker_details,
    require_printed_questions,
)
from pydantic import JsonValue, ValidationError
from rule_source_runtime import JsonObject, OperationFrame  # pyright: ignore[reportImplicitRelativeImport]


def test_protocol_errors_and_wrong_versions_cannot_count_as_operation_completion() -> (
    None
):
    valid: JsonObject = {
        "kind": "operation_result",
        "protocol_version": 1,
        "result": {"operation": "check_rules", "outcome": {"kind": "checked"}},
    }
    frame = OperationFrame.model_validate_json(json.dumps(valid))
    assert frame.result.operation == "check_rules"
    assert type(frame.protocol_version) is int and frame.protocol_version == 1
    for invalid in [
        {**valid, "protocol_version": True},
        {**valid, "protocol_version": False},
        {**valid, "protocol_version": 1.0},
        {**valid, "protocol_version": "1"},
        {**valid, "protocol_version": 0},
        {**valid, "protocol_version": 2},
        {**valid, "extra": True},
        {"kind": "protocol_error"},
    ]:
        with pytest.raises(ValidationError):
            _ = OperationFrame.model_validate_json(json.dumps(invalid))


def test_refusal_requires_actual_expected_source_and_positive_coordinates() -> None:
    definition: JsonObject = {
        "kind": "definition",
        "file": "/fixture/source.md",
        "line": 2,
        "column": 3,
    }
    source: JsonObject = {
        "kind": "source",
        "source": {"path": "source.md"},
        "span": {"start_line": 2, "start_column": 3},
    }
    for location in [definition]:
        require_located_refusal(
            {"kind": "refused", "diagnostics": {"items": [{"location": location}]}},
            "source.md",
        )
    invalid_locations: list[JsonObject] = [
        {**definition, "column": 0},
        {**definition, "file": "another.md"},
        {**definition, "file": "notsource.md"},
        {**definition, "column": True},
        {**definition, "source": {"path": "source.md"}},
        source,
        {"kind": "field", "field_path": ["source.md"]},
        {**source, "span": None},
    ]
    for location in invalid_locations:
        with pytest.raises(AssertionError):
            require_located_refusal(
                {"kind": "refused", "diagnostics": {"items": [{"location": location}]}},
                "source.md",
            )
    single: JsonObject = {
        "kind": "refused",
        "diagnostics": {"items": [{"location": definition}]},
    }
    with pytest.raises(AssertionError):
        require_located_refusal(single, "source.md", required_lines=frozenset({2, 3}))


def test_question_check_catches_swapped_text_wrong_target_and_missing_settings() -> (
    None
):
    expected: JsonObject = {
        name: f"independent question for {name}" for name in QUESTION_FILES
    }
    definitions: dict[str, JsonObject] = {
        name: {
            "target": "target",
            "applicability": [],
            "questions": {},
            "severity": "block" if name == "skill.call.complete" else "nudge",
            "verdict": {
                "kind": "noul",
                "question": {"instructions": expected[name]},
                "threshold": 0.8,
            },
            "evidence": [
                {
                    "id": "target",
                    "input": "subject",
                    "select": {"kind": "paragraph", "target_id": "U001"},
                }
            ],
        }
        for name in QUESTION_FILES
    }
    require_questions(definitions, expected)
    swapped = copy.deepcopy(definitions)
    swapped["skill.call.complete"]["verdict"] = definitions["skill.step.completion"][
        "verdict"
    ]
    with pytest.raises(AssertionError):
        require_questions(swapped, expected)
    mutations: list[tuple[str, JsonValue]] = [
        ("target", "wrong"),
        ("severity", "nudge"),
        ("evidence", []),
    ]
    for field, value in mutations:
        invalid = copy.deepcopy(definitions)
        invalid["skill.call.complete"][field] = value
        with pytest.raises((AssertionError, IndexError)):
            require_questions(invalid, expected)


def test_printed_output_cannot_omit_duplicate_or_misbind_a_checked_rule() -> None:
    definition: JsonObject = {
        "rule_id": "skill.one",
        "verdict": {"question": {"instructions": "second"}},
    }
    valid: JsonObject = {
        "kind": "checked",
        "checked_rule_count": 1,
        "printed_rules": [{"definition": definition}],
    }
    require_printed_questions(valid, {"skill.one": "second"})
    malformed: list[JsonObject] = [
        {**valid, "printed_rules": []},
        {**valid, "checked_rule_count": True},
        {
            **valid,
            "checked_rule_count": 2,
            "printed_rules": [{"definition": definition}, {"definition": definition}],
        },
    ]
    for invalid in malformed:
        with pytest.raises(AssertionError):
            require_printed_questions(invalid, {"skill.one": "second"})
    with pytest.raises(AssertionError):
        require_printed_questions(valid, {"skill.one": "- first\n\nsecond"})


def test_marker_refusal_cannot_lose_cause_identity_or_repair_guidance() -> None:
    diagnostic: JsonObject = {
        "code": "question_too_large",
        "guidance": MARKER_GUIDANCE["question_too_large"],
        "location": {"field_path": ["K014"]},
    }

    def outcome(item: JsonObject) -> JsonObject:
        return {"kind": "refused", "diagnostics": {"items": [item]}}

    require_marker_details(
        outcome(diagnostic), "question_too_large", frozenset({"K014"})
    )
    invalid_diagnostics: list[JsonObject] = [
        {**diagnostic, "code": "invalid_marker"},
        {**diagnostic, "guidance": "Repair the definition."},
        {**diagnostic, "location": {"field_path": []}},
        {**diagnostic, "location": {"field_path": ["K015"]}},
    ]
    for invalid in invalid_diagnostics:
        with pytest.raises(AssertionError):
            require_marker_details(
                outcome(invalid), "question_too_large", frozenset({"K014"})
            )
