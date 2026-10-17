"""US22: a required judgment must not disappear behind an unjustified unchanged result."""

from dataclasses import dataclass

from composition_scenarios import native_answer
from git_scenario_fixtures import BASE_DOCUMENT, paragraph_fixture
from scenario_fixtures import bindings, decision
from scenario_runtime import (
    DriverContext,
    JsonObject,
    OperationRequest,
    array_at,
    as_object,
    as_text,
    object_at,
    read_object,
    run_operation,
    write_object,
)
from scenario_transport import ControlledServer

GATE_TEXT = "U001: stable gate input\n"


@dataclass(frozen=True, kw_only=True)
class SelectionExpectation:
    kinds: tuple[str, str]
    reasons: tuple[list[str], list[str]]


def check_selection_pair(
    preparation: JsonObject, report: JsonObject, expected: SelectionExpectation
) -> None:
    planned = [
        as_object(value)
        for value in array_at(preparation, "preparation", "occurrences")
    ]
    observed = [as_object(value) for value in array_at(report, "occurrences")]
    assert len(planned) == len(observed) == 2
    assert [object_at(value, "key")["rule_id"] for value in observed] == [
        "changed",
        "neighbor",
    ]
    for index, (kind, reasons) in enumerate(zip(expected.kinds, expected.reasons)):
        assert observed[index]["kind"] == kind
        status = object_at(planned[index], "status")
        if kind == "judged":
            assert status["kind"] == "planned"
            assert object_at(status, "selection") == {
                "kind": "selected",
                "selected_by": reasons,
            }
            assert observed[index]["selected_by"] == reasons
            reference = object_at(
                observed[index], "judgment", "verdict", "basis", "answer"
            )
            assert native_answer(report, reference)["type"] == "noul"
        elif kind == "outside_change_scope":
            assert status == {
                "kind": "outside_change_scope",
                "reason": "no_declared_change",
            }
            assert observed[index]["reason"] == "no_declared_change"
        else:
            assert kind == "selection_failed" and status["kind"] == "selection_failed"
            assert "selected_by" not in observed[index]


def dependency_journey(context: DriverContext) -> None:
    summaries: list[JsonObject] = []
    for case in [
        "movement",
        "target",
        "context",
        "rule",
        "common",
        "baseline",
        "deleted",
        "applicability",
        "external",
        "gate-nonpass",
        "gate-pass",
    ]:
        with ControlledServer(context.directory / f"{case}-http") as server:
            repository, invocation = paragraph_fixture(
                context.directory / case, server.endpoint
            )
            root = repository.root
            definition = read_object(root / "set.json")
            changed_rule = read_object(root / "changed.jsonl")
            if case == "context":
                array_at(changed_rule, "evidence").append(
                    {
                        "id": "context",
                        "input": "subject",
                        "requirement": "required",
                        "select": {"kind": "paragraph", "target_id": "U002"},
                    }
                )
            if case == "applicability":
                changed_rule["applicability"] = [
                    {
                        "kind": "condition",
                        "test": {
                            "kind": "pattern_present",
                            "input": "subject",
                            "pattern": {"kind": "literal", "text": "original"},
                            "max_scan_bytes": 10000,
                        },
                    }
                ]
            if case == "external":
                external = context.directory / "external-context"
                external.mkdir()
                (external / "context.md").write_text("UNCHANGED_FULL_CONTEXT\n")
                object_at(definition, "inputs")["external"] = {"kind": "document"}
                bindings(invocation)["external"] = {
                    "kind": "document",
                    "repository": "external",
                    "path": "context.md",
                }
                array_at(invocation, "repositories").append(
                    {
                        "key": "external",
                        "root": str(external),
                        "input_paths": [{"kind": "exact", "path": "context.md"}],
                        "change_scope": {"kind": "full"},
                    }
                )
                array_at(changed_rule, "evidence").append(
                    {
                        "id": "context",
                        "input": "external",
                        "requirement": "required",
                        "select": {"kind": "whole_document"},
                    }
                )
            gate = case.startswith("gate-")
            if gate:
                object_at(definition, "inputs")["gate"] = {"kind": "document"}
                definition["applicability"] = [decision("gate")]
                bindings(invocation)["gate"] = {
                    "kind": "document",
                    "repository": "repo",
                    "path": "gate.md",
                }
                repository.write("gate.md", GATE_TEXT)
            write_object(root / "set.json", definition)
            write_object(root / "changed.jsonl", changed_rule)
            assert array_at(definition, "rule_paths") == [
                "changed.jsonl",
                "neighbor.jsonl",
            ]
            for path in ["set.json", "changed.jsonl", "neighbor.jsonl"]:
                assert (root / path).resolve().is_relative_to(root.resolve()) and (
                    root / path
                ).is_file()
            if repository.git("status", "--porcelain"):
                repository.commit(1100)
            expected = SelectionExpectation(
                kinds=("judged", "outside_change_scope"),
                reasons=(["target_changed"], []),
            )
            if case in ["movement", "gate-nonpass"]:
                repository.write("source.md", "# Prefix\n\n" + BASE_DOCUMENT)
                expected = SelectionExpectation(
                    kinds=("outside_change_scope", "outside_change_scope"),
                    reasons=([], []),
                )
            elif case in ["target", "gate-pass"]:
                repository.write(
                    "source.md", "U001: changed target\n\nU002: neighbor\n"
                )
            elif case in ["context", "applicability"]:
                repository.write("source.md", "U001: original\n\nU002: new context\n")
                reason = (
                    "context_changed" if case == "context" else "applicability_changed"
                )
                expected = SelectionExpectation(
                    kinds=("judged", "judged"), reasons=([reason], ["target_changed"])
                )
            elif case == "rule":
                object_at(changed_rule, "verdict", "question")["instructions"] = (
                    "UPDATED_RULE"
                )
                write_object(root / "changed.jsonl", changed_rule)
                expected = SelectionExpectation(
                    kinds=("judged", "outside_change_scope"),
                    reasons=(["rule_changed"], []),
                )
            elif case == "common":
                definition["common_ruleset_instructions"] = "UPDATED_COMMON"
                write_object(root / "set.json", definition)
                expected = SelectionExpectation(
                    kinds=("judged", "judged"),
                    reasons=(["rule_changed"], ["rule_changed"]),
                )
            elif case == "baseline":
                as_object(array_at(invocation, "repositories")[0])["change_scope"] = {
                    "kind": "revision_diff",
                    "base_revision": "missing-baseline",
                    "head": {"kind": "worktree"},
                }
                expected = SelectionExpectation(
                    kinds=("judged", "judged"),
                    reasons=(["baseline_unavailable"], ["baseline_unavailable"]),
                )
            elif case == "deleted":
                repository.write("source.md", "U002: neighbor\n")
                expected = SelectionExpectation(
                    kinds=("selection_failed", "outside_change_scope"), reasons=([], [])
                )
            elif case == "external":
                expected = SelectionExpectation(
                    kinds=("judged", "outside_change_scope"),
                    reasons=(["full_scope"], []),
                )
            exit_code = 3 if case == "deleted" else 0
            preparation = run_operation(
                context,
                OperationRequest(
                    name=f"{case}-prepare",
                    operation="prepare_rules",
                    input_value=invocation,
                    cwd=root,
                    expected_exit=exit_code,
                ),
            )
            assert server.count == 0
            if gate:
                occurrences = array_at(preparation, "preparation", "occurrences")
                assert len(occurrences) == 2
                assert all(
                    object_at(as_object(value), "status")
                    == {"kind": "planned", "selection": {"kind": "comparison_pending"}}
                    for value in occurrences
                )
                waiting = [
                    as_object(value)
                    for value in array_at(
                        preparation, "preparation", "planned_requests"
                    )
                    if object_at(as_object(value), "readiness")["kind"] == "waiting"
                ]
                assert waiting and all(
                    array_at(value, "readiness", "on") for value in waiting
                )
                assert object_at(preparation, "preparation", "measurements")[
                    "bytes_scanned"
                ] == len(GATE_TEXT.encode())
            with context.begin(
                OperationRequest(
                    name=f"{case}-evaluate",
                    operation="evaluate_rules",
                    input_value=invocation,
                    cwd=root,
                    expected_exit=exit_code,
                )
            ) as process:
                if gate:
                    pending = server.next()
                    assert len(object_at(pending.body, "questions")) == 1
                    assert (
                        object_at(
                            as_object(
                                next(
                                    iter(object_at(pending.body, "questions").values())
                                )
                            ),
                            "instructions",
                        )["owner"]
                        == "rule_set_decision:0"
                    )
                    pending.answer(0.1 if case == "gate-nonpass" else 0.9)
                judged = [
                    name
                    for name, kind in zip(["changed", "neighbor"], expected.kinds)
                    if kind == "judged"
                ]
                if judged and case != "gate-nonpass":
                    pending = server.next()
                    assert len(object_at(pending.body, "questions")) == len(judged)
                    evidence = object_at(pending.body, "state", "evidence")
                    labels = {name for name in evidence if name.endswith("/target")}
                    assert labels == {f"rule:{name}/target" for name in judged}
                    contents = object_at(pending.body, "state", "contents")
                    for name in judged:
                        target = object_at(evidence, f"rule:{name}/target")
                        text = contents[as_text(target["content"])]
                        expected_text = (
                            "U001: changed target"
                            if name == "changed" and case in ["target", "gate-pass"]
                            else "U001: original"
                            if name == "changed"
                            else "U002: new context"
                            if case in ["context", "applicability"]
                            else "U002: neighbor"
                        )
                        assert text == expected_text
                    if case in ["context", "external"]:
                        attribution = object_at(evidence, "rule:changed/context")
                        assert contents[as_text(attribution["content"])] == (
                            "U002: new context"
                            if case == "context"
                            else "UNCHANGED_FULL_CONTEXT\n"
                        )
                    if case == "common":
                        assert all(
                            object_at(as_object(value), "instructions")[
                                "common_ruleset_instructions"
                            ]
                            == "UPDATED_COMMON"
                            for value in object_at(pending.body, "questions").values()
                        )
                    if case == "rule":
                        assert (
                            object_at(
                                as_object(
                                    next(
                                        iter(
                                            object_at(
                                                pending.body, "questions"
                                            ).values()
                                        )
                                    )
                                ),
                                "instructions",
                            )["question"]
                            == "UPDATED_RULE"
                        )
                    pending.answer()
                outcome = process.finish(exit_code)
                report = object_at(outcome, "report")
            if case == "gate-nonpass":
                occurrences = array_at(report, "occurrences")
                assert len(occurrences) == 2
                assert all(
                    as_object(value)["kind"] == "not_applicable"
                    and "selected_by" not in as_object(value)
                    for value in occurrences
                )
                assert object_at(report, "measurements")["bytes_scanned"] == len(
                    GATE_TEXT.encode()
                )
            elif not gate:
                check_selection_pair(preparation, report, expected)
            else:
                occurrences = array_at(report, "occurrences")
                assert [as_object(value)["kind"] for value in occurrences] == [
                    "judged",
                    "outside_change_scope",
                ]
                assert as_object(occurrences[0])["selected_by"] == ["target_changed"]
            expected_http = (
                1 if case == "gate-nonpass" else 2 if gate else int(bool(judged))
            )
            assert server.count == expected_http
            assert object_at(report, "measurements")["model_requests"] == expected_http
            if case == "movement":
                assert array_at(preparation, "preparation", "planned_requests") == []
                requests = array_at(report, "requests")
                assert requests and all(
                    object_at(as_object(value), "status")
                    == {"kind": "not_sent", "reason": {"kind": "outside_change_scope"}}
                    for value in requests
                )
            if case == "deleted":
                assert "deleted_target" in str(report["diagnostics"])
            if case == "external":
                assert all(
                    as_object(value)["repository"] != "external"
                    for value in array_at(report, "change_selections")
                )
            summaries.append(
                {
                    "case": case,
                    "exit": exit_code,
                    "http_requests": server.count,
                    "occurrences": report["occurrences"],
                    "change_selections": report["change_selections"],
                }
            )
    write_object(context.directory / "dependency-readback.json", {"cases": summaries})
