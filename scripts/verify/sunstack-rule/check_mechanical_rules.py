# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic==2.14.1", "pydantic_core==2.50.1"]
# ///
"""Observe the CLI portion of U9 mechanical rules on a declared frozen S4 build.

Run from the monorepo root with uv run --script <this-file> --binary <binary>
--commit <full-HEAD> --evidence <new-owned-directory>. No provider credential is
inherited. An invalid backend endpoint makes mistaken backend construction fail.
Checks every expect form, frontmatter absence/unavailable outcomes, located
ast_grep refusal, and paragraph/list results versus full-document ast-grep 0.51.0
scan ranges. Original sources and independent scan tuples stay separate.

This driver does not make a runtime metadata target. Complete U9 additionally
requires the exact-target public full-tree metadata matcher, counted factory,
and staged/carried local-work proofs in the permanent native harness. Inspect
and reuse an applicable run; zero HTTP or this pass file alone cannot prove them.
"""

import copy
import hashlib
import json
import os
import re
import subprocess
from pathlib import Path

from check_rule_refusals import require_located_refusal  # pyright: ignore[reportImplicitRelativeImport]
from pydantic import JsonValue, TypeAdapter
from rule_source_runtime import (  # pyright: ignore[reportImplicitRelativeImport]
    REPOSITORY_ROOT,
    JourneyTarget,
    JsonObject,
    as_array,
    as_object,
    journey_target,
    object_at,
    read_json,
    source_fixture,
    write_json,
)

SCAN_MATCHES = TypeAdapter(list[JsonObject])
SCAN_ORACLE = Path(
    "crates/sunstack-rules/tests/fixtures/mechanical-ast/scan-oracle.json"
)


def mechanical_rule_entry(
    name: str, selector: JsonObject, check: JsonObject
) -> JsonObject:
    return {
        "name": name,
        "target": {"input": "subject", "select": selector},
        **check,
    }


def unit_selector(kind: str = "paragraph", prefix: str | None = None) -> JsonObject:
    value: JsonObject = {"kind": "markdown_units", "mode": "unit", "units": [kind]}
    if prefix is not None:
        value["starts_with"] = prefix
    return value


def runtime_input(check_input: JsonObject) -> JsonObject:
    value = copy.deepcopy(check_input)
    _ = value.pop("output")
    value.update(
        origin={"kind": "direct"},
        cursors=[],
        execution={"speculation": "off", "max_concurrent_requests": 1},
        backend={
            "kind": "jev_openrouter",
            "endpoint": "not-a-url",
            "model": "jev-latest",
        },
        attempts={
            "attempt_timeout_ms": 10000,
            "retry_count": 0,
            "initial_backoff_ms": 100,
            "max_backoff_ms": 1000,
            "max_response_bytes": 1048576,
        },
        request_budget={
            "max_input_tokens": 100000,
            "max_request_bytes": 262144,
            "max_questions": 1,
            "max_requests": 1,
            "safety_margin_tokens": 32,
            "accounting_policy": "allow_estimated",
        },
    )
    value["limits"] = {
        "max_files": 32,
        "max_file_bytes": 262144,
        "max_total_bytes": 4194304,
        "max_matches": 2048,
        "max_transcript_bytes": 262144,
        "include_ignored": False,
    }
    return value


def forbid_model_fields(value: JsonValue) -> None:
    if isinstance(value, dict):
        assert not {
            "judgment",
            "answer",
            "answers",
            "answer_ref",
            "answer_reference",
            "question_name",
            "request_index",
            "receipt",
            "confidence",
        }.intersection(value), "A local result fabricated model evidence"
        for child in value.values():
            forbid_model_fields(child)
    elif isinstance(value, list):
        for child in value:
            forbid_model_fields(child)


def local_report(outcome: JsonObject, *, complete: bool) -> JsonObject:
    assert outcome["kind"] == ("complete_report" if complete else "incomplete_report")
    report = object_at(outcome, "report")
    measurements = object_at(report, "measurements")
    for counter in ["model_requests", "inference_attempts"]:
        value = measurements[counter]
        assert type(value) is int and value == 0
    assert report["rule_coverage"] == "full"
    assert report["skipped_rules"] == []
    occurrences = as_array(report["occurrences"])
    assert occurrences, "An empty report cannot prove a local check"
    keys = [
        json.dumps(object_at(value, "key"), sort_keys=True) for value in occurrences
    ]
    assert len(set(keys)) == len(keys), "Local occurrence identity was duplicated"
    for value in occurrences:
        occurrence = as_object(value)
        assert occurrence["kind"] == "checked"
        forbid_model_fields(occurrence)
    for value in as_array(report["requests"]):
        request = as_object(value)
        assert request["questions"] == []
        assert object_at(request, "status")["kind"] == "not_sent"
        assert object_at(request, "status", "reason")["kind"] == "no_questions"
    return report


def sole_rule(report: JsonObject, name: str) -> JsonObject:
    found = [
        as_object(value)
        for value in as_array(report["occurrences"])
        if object_at(value, "key")["rule_id"] == name
    ]
    assert len(found) == 1, f"Expected exactly one {name} occurrence"
    return found[0]


def require_completed(occurrence: JsonObject, expected: str) -> None:
    result = object_at(occurrence, "outcome")
    assert result["kind"] == "completed" and result["result"] == expected


def observe_expect_forms(target: JourneyTarget, fixtures: Path) -> list[str]:
    passed: list[str] = []
    forms: list[tuple[str, JsonObject, str, str]] = [
        (
            "matches",
            {"matches": r"\breturn\b"},
            "Use a step and return proof.\n",
            "Use a step only.\n",
        ),
        (
            "not-matches",
            {"not_matches": r"\bsecret\b"},
            "Use public input.\n",
            "Use secret input.\n",
        ),
        (
            "starts-with",
            {"starts_with": "Use "},
            "Use explicit input.\n",
            "Avoid ambiguity.\n",
        ),
    ]
    for name, predicate, good, bad in forms:
        for label, text, expected in [("good", good, "pass"), ("bad", bad, "fail")]:
            case = f"expect-{name}-{label}"
            root = fixtures / case
            invocation = source_fixture(
                root,
                "Require explicit local behavior.<!-- K014 -->\n",
                {
                    "K014": mechanical_rule_entry(
                        "local.expect", unit_selector(), {"expect": predicate}
                    )
                },
            )
            _ = (root / "reviewed.md").write_text(text)
            outcome = target.run(
                case, "evaluate_rules", runtime_input(invocation), expected_exit=0
            )
            checked = sole_rule(local_report(outcome, complete=True), "local.expect")
            require_completed(checked, expected)
            assert checked["declared_severity"] == "nudge"
            passed.append(case)
    return passed


def observe_frontmatter(target: JourneyTarget, fixtures: Path) -> list[str]:
    passed: list[str] = []
    cases: list[tuple[str, str, int, str | None]] = [
        ("present-null", "---\ndescription: null\n---\n\nUse peer.\n", 0, "pass"),
        ("absent", "---\nname: other\n---\n\nUse peer.\n", 0, "fail"),
        ("missing", "Use peer.\n", 3, None),
        ("malformed", "---\ndescription: [\n---\n\nUse peer.\n", 3, None),
    ]
    for name, text, exit_code, expected in cases:
        root = fixtures / f"field-{name}"
        field = mechanical_rule_entry(
            "local.description",
            {"kind": "frontmatter_field", "field": "description"},
            {"expect": {"present": True}},
        )
        field["severity"] = "block"
        invocation = source_fixture(
            root,
            "Require a description.<!-- K014 -->\n\nKeep a peer.<!-- K015 -->\n",
            {
                "K014": field,
                "K015": mechanical_rule_entry(
                    "local.peer",
                    unit_selector(prefix="Use peer"),
                    {"expect": {"starts_with": "Use"}},
                ),
            },
        )
        _ = (root / "reviewed.md").write_text(text)
        case = f"field-{name}"
        outcome = target.run(
            case, "evaluate_rules", runtime_input(invocation), expected_exit=exit_code
        )
        report = local_report(outcome, complete=exit_code == 0)
        checked = sole_rule(report, "local.description")
        assert checked["declared_severity"] == "block"
        if expected is not None:
            require_completed(checked, expected)
        else:
            assert object_at(checked, "outcome")["kind"] == "unresolved"
            assert as_array(object_at(checked, "outcome", "diagnostics")["items"])
        require_completed(sole_rule(report, "local.peer"), "pass")
        passed.append(case)
    return passed


def observe_decoded_fields(target: JourneyTarget, fixtures: Path) -> list[str]:
    passed: list[str] = []
    values: list[tuple[str, JsonValue]] = [
        ("string", "Use when café"),
        ("number", 4),
        ("boolean", True),
        ("null", None),
        ("array", ["one", 2]),
        ("object", {"b": 2, "a": 1}),
    ]
    for label, value in values:
        canonical = (
            value
            if isinstance(value, str)
            else json.dumps(
                value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
            )
        )
        predicates: list[JsonObject] = [
            {"matches": "^" + re.escape(canonical) + "$"},
            {"starts_with": canonical},
        ]
        case = f"decoded-{label}"
        root = fixtures / case
        entries: JsonObject = {}
        teaching: list[str] = []
        for index, predicate in enumerate(predicates, start=14):
            marker = f"K{index:03d}"
            entries[marker] = mechanical_rule_entry(
                f"local.decoded{index}",
                {"kind": "frontmatter_field", "field": "description"},
                {"expect": predicate},
            )
            teaching.append(f"Match the decoded field.<!-- {marker} -->\n")
        invocation = source_fixture(root, "\n".join(teaching), entries)
        _ = (root / "reviewed.md").write_text(
            "---\ndescription: " + json.dumps(value, ensure_ascii=False) + "\n---\n"
        )
        outcome = target.run(
            case, "evaluate_rules", runtime_input(invocation), expected_exit=0
        )
        report = local_report(outcome, complete=True)
        assert len(as_array(report["occurrences"])) == 2
        for index in [14, 15]:
            require_completed(sole_rule(report, f"local.decoded{index}"), "pass")
        passed.append(case)
    root = fixtures / "absent-all-forms"
    predicates = [
        {"matches": "Use"},
        {"not_matches": "forbidden"},
        {"starts_with": "Use"},
        {"present": True},
    ]
    entries = {}
    teaching = []
    for index, predicate in enumerate(predicates, start=14):
        marker = f"K{index:03d}"
        entries[marker] = mechanical_rule_entry(
            f"local.absent{index}",
            {"kind": "frontmatter_field", "field": "description"},
            {"expect": predicate},
        )
        teaching.append(f"Require an existing field.<!-- {marker} -->\n")
    invocation = source_fixture(root, "\n".join(teaching), entries)
    _ = (root / "reviewed.md").write_text("---\nname: other\n---\n")
    outcome = target.run(
        "absent-all-forms", "evaluate_rules", runtime_input(invocation), expected_exit=0
    )
    report = local_report(outcome, complete=True)
    assert len(as_array(report["occurrences"])) == 4
    for index in range(14, 18):
        require_completed(sole_rule(report, f"local.absent{index}"), "fail")
    passed.append("absent-all-forms")
    return passed


def observe_invalid_ast(target: JourneyTarget, fixtures: Path) -> str:
    root = fixtures / "invalid-ast"
    invocation = source_fixture(root, "Reject invalid AST rules.<!-- K014 -->\n", {})
    rules = '{\n  "K014": {\n    "name": "local.invalid",\n    "target": {"input":"subject","select":{"kind":"markdown_units","mode":"unit","units":["paragraph"]}},\n    "ast_grep": {"regex":"["}\n  }\n}\n'
    _ = (root / "source.rules.json").write_text(rules)
    assert not (root / "reviewed.md").exists()
    outcome = target.run("invalid-ast", "check_rules", invocation, expected_exit=2)
    require_located_refusal(outcome, "source.rules.json", required_lines=frozenset({5}))
    diagnostics = as_array(object_at(outcome, "diagnostics")["items"])
    assert any(
        as_object(item)["code"] == "invalid_mechanical_check" for item in diagnostics
    )
    return "invalid-ast"


def observe_nested_ast_refusal(target: JourneyTarget, fixtures: Path) -> str:
    root = fixtures / "nested-unknown-ast"
    invocation = source_fixture(
        root, "Reject misspelled relation keys.<!-- K014 -->\n", {}
    )
    rules = '{\n  "K014": {\n    "name": "local.nested",\n    "target": {"input":"subject","select":{"kind":"markdown_units","mode":"unit","units":["paragraph"],"starts_with":"nested"}},\n    "ast_grep": {"kind":"paragraph","regex":"MUST","inside":{"kind":"list_item","regex":"outer","stop_by":"end"}}\n  }\n}\n'
    _ = (root / "source.rules.json").write_text(rules)
    assert not (root / "reviewed.md").exists()
    outcome = target.run(
        "nested-unknown-ast", "check_rules", invocation, expected_exit=2
    )
    require_located_refusal(outcome, "source.rules.json", required_lines=frozenset({5}))
    checked_count = outcome["checked_rule_count"]
    assert type(checked_count) is int and checked_count == 0
    diagnostics = as_array(object_at(outcome, "diagnostics")["items"])
    assert any(
        as_object(item)["code"] == "unknown_field"
        and object_at(item, "location")["field_path"]
        == ["ast_grep", "inside", "stop_by"]
        for item in diagnostics
    ), "Nested typo lost its unknown-field cause or JSON path"
    return "nested-unknown-ast"


def scan_environment() -> dict[str, str]:
    return {
        key: os.environ[key]
        for key in ["PATH", "HOME", "TMPDIR", "LANG"]
        if key in os.environ
    }


def run_scan(
    target: JourneyTarget, name: str, source: Path, rule: JsonObject
) -> list[JsonObject]:
    target.confirm()
    directory = target.evidence / name
    directory.mkdir()
    command = [
        "ast-grep",
        "scan",
        "--inline-rules",
        json.dumps(
            {
                "id": "reference",
                "language": "Markdown",
                "severity": "warning",
                "message": "reference",
                "rule": rule,
            }
        ),
        "--json=compact",
        "--color=never",
        str(source),
    ]
    environment = scan_environment()
    with (
        (directory / "stdout.json").open("wb") as out,
        (directory / "stderr.log").open("wb") as err,
    ):
        result = subprocess.run(
            command,
            cwd=REPOSITORY_ROOT,
            env=environment,
            stdout=out,
            stderr=err,
            check=False,
            timeout=30,
        )
    write_json(
        directory / "result.json",
        {
            "command": [argument for argument in command],
            "exit_code": result.returncode,
            "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
        },
    )
    assert result.returncode == 0
    matches: list[JsonObject] = []
    for value in SCAN_MATCHES.validate_json((directory / "stdout.json").read_bytes()):
        span = object_at(value, "range", "byteOffset")
        assert type(span["start"]) is int and type(span["end"]) is int
        matches.append(
            {"start": span["start"], "end": span["end"], "text": value["text"]}
        )
    target.confirm()
    return matches


def contained_matches(matches: list[JsonObject], span: JsonObject) -> list[JsonObject]:
    start, end = span["start"], span["end"]
    assert type(start) is int and type(end) is int
    found: list[JsonObject] = []
    for match in matches:
        left, right = match["start"], match["end"]
        assert type(left) is int and type(right) is int
        if start <= left and right <= end:
            found.append(match)
    return found


def observe_ast_corpus(target: JourneyTarget, fixtures: Path) -> list[str]:
    oracle = read_json(REPOSITORY_ROOT / SCAN_ORACLE)
    assert oracle["cli_version"] == "ast-grep 0.51.0"
    version = subprocess.check_output(
        ["ast-grep", "--version"], text=True, timeout=10, env=scan_environment()
    ).strip()
    assert version == oracle["cli_version"]
    rules = as_object(oracle["rules"])
    passed: list[str] = []
    runtime_files = positive_units = ancestor_units = 0
    for value in as_array(oracle["cases"]):
        case = as_object(value)
        source_name = case["source"]
        assert isinstance(source_name, str)
        source = REPOSITORY_ROOT / source_name
        raw = source.read_bytes()
        assert hashlib.sha256(raw).hexdigest() == case["sha256"]
        reference: dict[str, list[JsonObject]] = {}
        for rule_name, rule in rules.items():
            matches = run_scan(
                target, f"scan-{source.stem}-{rule_name}", source, as_object(rule)
            )
            expected = [
                as_object(item)
                for item in as_array(object_at(case, "matches")[rule_name])
            ]
            assert matches == expected, (
                "Full-document scan differs from committed independent oracle"
            )
            for match in matches:
                left, right = match["start"], match["end"]
                assert type(left) is int and type(right) is int
                assert raw[left:right].decode() == match["text"]
            reference[rule_name] = matches
        if source.stem in {"metadata-pass", "metadata-fail"}:
            continue
        runtime_files += 1
        root = fixtures / f"ast-{source.stem}"
        entries: JsonObject = {}
        teaching: list[str] = []
        for index, (name, unit, check_name) in enumerate(
            [
                ("local.call", "paragraph", "call_paragraph"),
                ("local.inside.paragraph", "paragraph", "inside_list"),
                ("local.inside.item", "list_item", "inside_list"),
            ],
            start=14,
        ):
            marker = f"K{index:03d}"
            entries[marker] = mechanical_rule_entry(
                name, unit_selector(unit), {"ast_grep": rules[check_name]}
            )
            teaching.append(f"Apply the structural standard.<!-- {marker} -->\n")
        invocation = source_fixture(root, "\n".join(teaching), entries)
        _ = (root / "reviewed.md").write_bytes(raw)
        name = f"ast-{source.stem}"
        outcome = target.run(
            name, "evaluate_rules", runtime_input(invocation), expected_exit=0
        )
        report = local_report(outcome, complete=True)
        for value in as_array(report["occurrences"]):
            occurrence = as_object(value)
            key = object_at(occurrence, "key")
            identifier = key["rule_id"]
            assert identifier in {
                "local.call",
                "local.inside.paragraph",
                "local.inside.item",
            }
            selected = object_at(key, "target")
            assert selected["kind"] == "markdown_unit"
            expected = contained_matches(
                reference[
                    "call_paragraph" if identifier == "local.call" else "inside_list"
                ],
                object_at(selected, "range"),
            )
            settlement = object_at(occurrence, "outcome")
            assert settlement["kind"] == "completed"
            actual: list[JsonObject] = []
            for observation in as_array(settlement["observations"]):
                assert as_object(observation)["kind"] == "ast_match"
                span = object_at(observation, "span")
                left, right = span["byte_start"], span["byte_end"]
                assert type(left) is int and type(right) is int
                actual.append(
                    {"start": left, "end": right, "text": raw[left:right].decode()}
                )
            assert actual == expected
            assert settlement["result"] == ("fail" if expected else "pass")
            positive_units += bool(expected)
            ancestor_units += identifier == "local.inside.item" and bool(expected)
        passed.append(name)
    assert runtime_files == 7 and positive_units > 0 and ancestor_units > 0
    return passed


def main() -> None:
    target = journey_target(__doc__ or "Mechanical runtime checks")
    fixtures = target.evidence / "fixtures"
    fixtures.mkdir()
    passed = observe_expect_forms(target, fixtures)
    passed.extend(observe_frontmatter(target, fixtures))
    passed.extend(observe_decoded_fields(target, fixtures))
    passed.append(observe_invalid_ast(target, fixtures))
    passed.append(observe_nested_ast_refusal(target, fixtures))
    passed.extend(observe_ast_corpus(target, fixtures))
    target.confirm()
    write_json(
        target.evidence / "journey-pass.json",
        {
            "journey": "mechanical rules CLI controls",
            "commit": target.commit,
            "cases": [name for name in passed],
            "provider_credential_present": False,
            "native_proof_obligations": [
                "metadata matcher explicit full-tree ranges",
                "counted backend factory zero/one/error controls",
                "staged/carried local gates, cancellation and budget",
            ],
        },
    )
    print(
        f"mechanical CLI: {len(passed)} cases passed; inspect native proof obligations; {target.evidence}"
    )


if __name__ == "__main__":
    main()
