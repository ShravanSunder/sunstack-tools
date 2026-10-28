# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic==2.14.1", "pydantic_core==2.50.1"]
# ///
"""Check Markdown rule-source refusals through a declared sunstack exec process.

Run from the monorepo root with uv run --script <this-file> --binary <binary>
--commit <full-HEAD> --evidence <new-owned-directory>. Needs the check_rules
operation at the R37 cutover; no provider credential, backend or paid call is used. Preserves
each input, stdout, stderr and exit. Oracle: Markdown-rule Specification R1–R13
and Verification Design U4. This is admission proof, not judgment-quality proof.
"""

import copy
import json
from dataclasses import dataclass
from pathlib import Path

from pydantic import JsonValue

# Executed by file path: this directory is Python's script import root.
from rule_source_runtime import (  # pyright: ignore[reportImplicitRelativeImport]
    JourneyTarget,
    JsonObject,
    as_array,
    as_object,
    bound_evaluation_input,
    check_source_fixture,
    journey_target,
    local_evaluation_input,
    object_at,
    printed_rule_definitions,
    read_json,
    rule_entry,
    write_json,
)


@dataclass(frozen=True, kw_only=True)
class RefusalCase:
    name: str
    markdown: str
    rules: JsonObject
    file: str = "source.md"
    max_question_words: int | None = 400
    duplicate_json_key: bool = False


def refusal_cases() -> list[RefusalCase]:
    one_rule: JsonObject = {"K014": rule_entry("skill.one")}
    cases = [
        RefusalCase(name=name, markdown=text, rules=copy.deepcopy(one_rule))
        for name, text in [
            ("h1", "# Heading<!-- K014 -->\n"),
            ("block-quote", "> prose<!-- K014 -->\n"),
            ("nested-quote", "> - prose<!-- K014 -->\n"),
            ("table", "| name |\n| --- |\n| text<!-- K014 -->\n"),
            ("html-block", "<div>\ntext<!-- K014 -->\n</div>\n"),
            ("frontmatter", "---\nname: text<!-- K014 -->\n---\n"),
            ("link-definition", "[ref]: /path <!-- K014 -->\n"),
            ("footnote", "[^ref]: text<!-- K014 -->\n"),
            ("spaced-heading", "## Heading <!-- K014 -->\n"),
            ("duplicate-marker", "one<!-- K014 -->\n\ntwo<!-- K014 -->\n"),
        ]
    ]
    cases.extend(
        [
            RefusalCase(
                name="empty-source-pair",
                markdown="# No marked standards\n",
                rules={},
                file="source.rules.json",
            ),
            RefusalCase(
                name="missing-entry", markdown="prose<!-- K014 -->\n", rules={}
            ),
            RefusalCase(
                name="orphan-entry",
                markdown="prose\n",
                rules=copy.deepcopy(one_rule),
                file="source.rules.json",
            ),
            RefusalCase(
                name="four-digit-non-marker",
                markdown="prose<!-- K0014 -->\n",
                rules=copy.deepcopy(one_rule),
                file="source.rules.json",
            ),
            RefusalCase(
                name="oversize",
                markdown="one two three<!-- K014 -->\n",
                rules=copy.deepcopy(one_rule),
                max_question_words=2,
            ),
            RefusalCase(
                name="oversize-default-cap",
                markdown="word " * 401 + "<!-- K014 -->\n",
                rules=copy.deepcopy(one_rule),
                max_question_words=None,
            ),
            RefusalCase(
                name="duplicate-json-key",
                markdown="prose<!-- K014 -->\n",
                rules=copy.deepcopy(one_rule),
                file="source.rules.json",
                duplicate_json_key=True,
            ),
        ]
    )
    mutations: list[tuple[str, str, JsonValue]] = [
        ("unknown-key", "unknown_key", True),
        ("bad-threshold", "threshold", 1.1),
        (
            "invalid-regex",
            "context",
            [
                {
                    "id": "regex_context",
                    "input": "subject",
                    "select": {"kind": "regex_text", "pattern": "["},
                }
            ],
        ),
    ]
    for name, field, value in mutations:
        rules = copy.deepcopy(one_rule)
        as_object(rules["K014"])[field] = value
        cases.append(
            RefusalCase(
                name=name,
                markdown="prose<!-- K014 -->\n",
                rules=rules,
                file="source.rules.json",
            )
        )
    return cases


def require_located_refusal(
    outcome: JsonObject, filename: str, *, required_lines: frozenset[int] | None = None
) -> None:
    assert outcome["kind"] == "refused", "Expected an admission refusal"
    diagnostics = as_array(object_at(outcome, "diagnostics")["items"])
    assert diagnostics, "No refusal diagnostic"
    locations = [object_at(diagnostic, "location") for diagnostic in diagnostics]
    assert all(
        location.get("kind") == "definition" and "source" not in location
        for location in locations
    ), "Rule definition diagnostics must not fabricate evidence SourceRefs"
    located = [
        location for location in locations if located_in_file(location, filename)
    ]
    assert located, f"No file:line:column for {filename}: {locations}"
    observed_lines: set[int] = set()
    for location in located:
        line = location["line"]
        assert type(line) is int
        observed_lines.add(line)
    if required_lines is not None:
        assert required_lines <= observed_lines, (
            "Check omitted an independently invalid marker"
        )


def require_retired_refusal(outcome: JsonObject, field_path: list[str]) -> None:
    assert outcome["kind"] == "refused"
    assert outcome["checked_rule_count"] == 0
    diagnostics = as_array(object_at(outcome, "diagnostics")["items"])
    assert any(
        as_object(item)["code"] == "unknown_field"
        and object_at(item, "location")["kind"] == "definition"
        and object_at(item, "location")["field_path"] == field_path
        for item in diagnostics
    ), f"Retired request field was not named: {field_path}"


def require_runtime_discovery_refusal(outcome: JsonObject) -> None:
    assert outcome["kind"] == "invalid"
    diagnostics = as_array(object_at(outcome, "diagnostics")["items"])
    assert diagnostics and all(
        as_object(item)["code"] == "discovery_not_enabled"
        and as_object(item)["origin"] == "invocation"
        and object_at(item, "location") == {"kind": "none"}
        for item in diagnostics
    ), "Missing runtime discovery must retain its invocation-level cause"


def located_in_file(location: JsonObject, filename: str) -> bool:
    match location.get("kind"):
        case "definition":
            line, column = location.get("line"), location.get("column")
            return Path(str(location.get("file", ""))).name == filename and (
                type(line) is int and line >= 1 and type(column) is int and column >= 1
            )
        case _:
            return False


MARKER_GUIDANCE: dict[str, str] = {
    "forbidden_marker_host": "Move the marker to a heading of level 2–6, a paragraph or a list item, outside block quotes, tables and HTML.",
    "spaced_heading_marker": "Remove the space before the heading marker.",
    "duplicate_marker": "Use each marker ID at most once in a Markdown source.",
    "question_too_large": "Mark a narrower host, such as a subsection, a paragraph or a list item.",
    "missing_rule_entry": "Add a rules entry for this marker ID.",
    "orphaned_rule_entry": "Add the corresponding marker or remove the orphaned rules entry.",
}
CASE_MARKER_CODES: dict[str, str] = {
    **{
        name: "forbidden_marker_host"
        for name in [
            "h1",
            "block-quote",
            "nested-quote",
            "table",
            "html-block",
            "frontmatter",
            "link-definition",
            "footnote",
        ]
    },
    "spaced-heading": "spaced_heading_marker",
    "duplicate-marker": "duplicate_marker",
    "missing-entry": "missing_rule_entry",
    "orphan-entry": "orphaned_rule_entry",
    "four-digit-non-marker": "orphaned_rule_entry",
    "oversize": "question_too_large",
    "oversize-default-cap": "question_too_large",
}


def require_marker_details(
    outcome: JsonObject, code: str, marker_ids: frozenset[str]
) -> None:
    diagnostics = as_array(object_at(outcome, "diagnostics")["items"])
    found: set[str] = set()
    for value in diagnostics:
        diagnostic = as_object(value)
        assert diagnostic["code"] == code, "Marker refusal lost its specific cause"
        assert diagnostic["guidance"] == MARKER_GUIDANCE[code], "Wrong repair guidance"
        field_path = object_at(diagnostic, "location")["field_path"]
        assert isinstance(field_path, list) and len(field_path) == 1
        marker = field_path[0]
        assert isinstance(marker, str) and marker in marker_ids
        found.add(marker)
    assert found == marker_ids, "Marker identity was omitted from refusal diagnostics"


def check_discovery_admission(target: JourneyTarget, fixtures: Path) -> list[str]:
    root = fixtures / "bounded-discovery"
    entry = rule_entry("skill.connected")
    entry["target"] = {
        "input": "changes",
        "select": {
            "kind": "changed_units",
            "languages": ["typescript"],
            "unit_kinds": ["function"],
            "source_sides": "after_only",
        },
    }
    invocation = check_source_fixture(
        root, "Connected standard<!-- K014 -->\n", {"K014": entry}
    )
    header = read_json(root / "set.json")
    object_at(header, "inputs")["changes"] = {"kind": "change_set"}
    write_json(root / "set.json", header)
    assert not (root / "reviewed.md").exists() and not (root / ".git").exists()
    outcome = target.run(
        "definition-selector", "check_rules", invocation, expected_exit=0
    )
    require_printed_questions(outcome, {"skill.connected": "Connected standard"})
    definitions = printed_rule_definitions(outcome)
    declaration = as_object(as_array(definitions["skill.connected"]["evidence"])[0])
    assert object_at(declaration, "select")["kind"] == "changed_units"
    runtime = local_evaluation_input(bound_evaluation_input(root))
    group = as_object(as_array(object_at(runtime, "definitions")["rule_sets"])[0])
    bindings = object_at(as_array(group["binding_groups"])[0], "bindings")
    bindings["changes"] = {"kind": "change_set", "repository": "repo"}
    outcome = target.run(
        "discovery-default-none", "evaluate_rules", runtime, expected_exit=2
    )
    # A22 moves this check to runtime preflight. The retired checker attached a
    # source location; runtime discovery is an invocation setting, not a rule.
    require_runtime_discovery_refusal(outcome)
    assert not (root / "reviewed.md").exists() and not (root / ".git").exists()
    return ["definition-selector", "discovery-default-none"]


def require_printed_questions(outcome: JsonObject, expected: JsonObject) -> None:
    definitions = printed_rule_definitions(outcome)
    questions: JsonObject = {
        name: object_at(definition, "verdict", "question")["instructions"]
        for name, definition in definitions.items()
    }
    assert questions == expected, "Printed bindings differ from source-visible text"


def check_retired_requests(target: JourneyTarget, fixtures: Path) -> list[str]:
    root = fixtures / "retired-check-inputs"
    valid = check_source_fixture(
        root, "A standard.<!-- K014 -->\n", {"K014": rule_entry("skill.one")}
    )
    repositories = copy.deepcopy(valid)
    repositories["repositories"] = []
    discovery = copy.deepcopy(valid)
    discovery["discovery"] = {"kind": "none"}
    bound_groups = copy.deepcopy(valid)
    bound_groups["definitions"] = bound_evaluation_input(root)["definitions"]
    retired = bound_evaluation_input(root)
    retired["output"] = "assembled_rules"
    cases: list[tuple[str, JsonObject, list[str]]] = [
        ("retired-repositories", repositories, ["repositories"]),
        ("retired-discovery", discovery, ["discovery"]),
        (
            "retired-binding-groups",
            bound_groups,
            ["definitions", "rule_sets", "0", "binding_groups"],
        ),
        ("retired-bound-request", retired, ["repositories"]),
    ]
    for name, request, field_path in cases:
        outcome = target.run(name, "check_rules", request, expected_exit=2)
        require_retired_refusal(outcome, field_path)
    assert not (root / "reviewed.md").exists()
    return [name for name, _, _ in cases]


def main() -> None:
    target = journey_target(__doc__ or "Check rule refusals")
    passed: list[str] = []
    fixtures = target.evidence / "fixtures"
    fixtures.mkdir()
    for case in refusal_cases():
        root = fixtures / case.name
        invocation = check_source_fixture(root, case.markdown, case.rules)
        ruleset = read_json(root / "set.json")
        if case.max_question_words is not None:
            ruleset["max_question_words"] = case.max_question_words
        write_json(root / "set.json", ruleset)
        if case.duplicate_json_key:
            entry = json.dumps(case.rules["K014"])
            _ = (root / "source.rules.json").write_text(
                f'{{"K014": {entry}, "K014": {entry}}}\n'
            )
        outcome = target.run(case.name, "check_rules", invocation, expected_exit=2)
        require_located_refusal(outcome, case.file)
        if case.name == "empty-source-pair":
            diagnostics = as_array(object_at(outcome, "diagnostics")["items"])
            assert len(diagnostics) == 1
            diagnostic = as_object(diagnostics[0])
            assert diagnostic["code"] == "empty_rule_file"
            location = object_at(diagnostic, "location")
            assert location["line"] == 1 and location["column"] == 1
            assert outcome["checked_rule_count"] == 0
        code = CASE_MARKER_CODES.get(case.name)
        if code is not None:
            require_marker_details(outcome, code, frozenset({"K014"}))
        passed.append(case.name)

    valid_cases: list[tuple[str, str, str]] = [
        (
            "clean",
            "## Heading<!-- K014 -->\n\nbody\n",
            "Heading\n\nbody",
        ),
        (
            "nested-list",
            "- parent<!-- K014 -->\n  - child\n- sibling\n",
            "- parent\n  - child",
        ),
        (
            "later-list-paragraph",
            "- first\n\n  second<!-- K014 -->\n",
            "second",
        ),
        (
            "ignored-comments",
            "real<!-- K014 -->\n\nLiteral \\<!-- K014 -->\n\n`code <!-- K015 -->`\n\n```md\nfence<!-- K016 -->\n```\n\nprose<!-- K017 --> trailing\n\nprose<!-- K0014 -->\n",
            "real",
        ),
    ]
    for name, markdown, question in valid_cases:
        invocation = check_source_fixture(
            fixtures / name, markdown, {"K014": rule_entry("skill.one")}
        )
        outcome = target.run(name, "check_rules", invocation, expected_exit=0)
        require_printed_questions(outcome, {"skill.one": question})
        passed.append(name)

    invocation = check_source_fixture(
        fixtures / "all-refusals",
        "## First <!-- K014 -->\n\n## Second <!-- K015 -->\n",
        {"K014": rule_entry("skill.first"), "K015": rule_entry("skill.second")},
    )
    outcome = target.run("all-refusals", "check_rules", invocation, expected_exit=2)
    require_located_refusal(outcome, "source.md", required_lines=frozenset({1, 3}))
    assert outcome["refused_rule_count"] == 2
    require_marker_details(
        outcome, "spaced_heading_marker", frozenset({"K014", "K015"})
    )
    passed.append("all-refusals")

    root = fixtures / "source-qualified-id"
    invocation = check_source_fixture(
        root, "first<!-- K014 -->\n", {"K014": rule_entry("skill.first")}
    )
    _ = (root / "other.md").write_text("second<!-- K014 -->\n")
    write_json(root / "other.rules.json", {"K014": rule_entry("skill.second")})
    ruleset = read_json(root / "set.json")
    as_array(ruleset["rule_sources"]).append(
        {"markdown": "other.md", "rules": "other.rules.json"}
    )
    write_json(root / "set.json", ruleset)
    outcome = target.run(
        "source-qualified-id", "check_rules", invocation, expected_exit=0
    )
    require_printed_questions(
        outcome, {"skill.first": "first", "skill.second": "second"}
    )
    passed.append("source-qualified-id")
    passed.extend(check_discovery_admission(target, fixtures))
    passed.extend(check_retired_requests(target, fixtures))
    target.confirm()
    write_json(
        target.evidence / "journey-pass.json",
        {
            "journey": "check refusals",
            "commit": target.commit,
            "cases": [name for name in passed],
            "provider_credential_present": False,
            "backend_construction_and_dispatch": "requires independent source-wiring observation",
        },
    )
    print(f"check refusals: {len(passed)} cases passed; evidence {target.evidence}")


if __name__ == "__main__":
    main()
