"""Caller-owned preparation, grouping, applicability and routing compositions."""

import copy
import json

from scenario_fixtures import (
    base_invocation,
    bindings,
    conversation_selection,
    decision,
    document_binding,
    noul_rule,
    ruleset,
    simple_fixture,
)
from scenario_runtime import (
    DriverContext,
    JsonObject,
    OperationRequest,
    array_at,
    as_object,
    as_text,
    complete_report,
    object_at,
    read_object,
    run_operation,
    write_object,
)
from scenario_transport import ControlledServer


def objects(value: JsonObject, name: str) -> list[JsonObject]:
    return [as_object(item) for item in array_at(value, name)]


def check_counts(report: JsonObject, *, requests: int, files: int) -> None:
    measurements = object_at(report, "measurements")
    assert measurements["model_requests"] == requests
    assert measurements["files_read"] == files


def native_answer(report: JsonObject, reference: JsonObject) -> JsonObject:
    index = reference["request_index"]
    name = reference["question_name"]
    assert isinstance(index, int) and isinstance(name, str)
    request = objects(report, "requests")[index]
    return object_at(request, "status", "outcome", "answers", name)


def request_owner_label(body: JsonObject) -> str:
    text = json.dumps(body)
    questions = list(object_at(body, "questions").values())
    assert len(questions) == 1
    common = object_at(as_object(questions[0]), "instructions").get(
        "common_ruleset_instructions"
    )
    target = object_at(body, "state", "evidence", "rule:shared/target")
    content = object_at(body, "state", "contents")[as_text(target["content"])]
    matches = [
        label
        for label in ["A", "B"]
        if common == f"OWNER_{label}" and content == f"U001: DOCUMENT_{label}"
    ]
    assert len(matches) == 1, "Missing or mismatched owner instructions/document"
    label = matches[0]
    other = "B" if label == "A" else "A"
    assert f"DOCUMENT_{label}" in text and f"DOCUMENT_{other}" not in text
    assert f"OWNER_{other}" not in text
    return label


def request_router_group(body: JsonObject) -> int:
    text = json.dumps(body)
    assert "CHAT_GROUP" not in text
    instructions = [
        object_at(as_object(question), "instructions")
        for question in object_at(body, "questions").values()
    ]
    routers = [item for item in instructions if item.get("owner") == "router"]
    assert len(routers) == 1 and "router/context" in array_at(routers[0], "reads")
    metadata = object_at(body, "state", "evidence", "router/context")
    assert metadata["input"] == "document" and metadata["kind"] == "document_text"
    content = object_at(body, "state", "contents")[as_text(metadata["content"])]
    matches = [group for group in [0, 1] if content == f"U001: DOC_GROUP_{group}\n"]
    assert len(matches) == 1, "Missing router-owned group document"
    group = matches[0]
    assert f"DOC_GROUP_{1 - group}" not in text
    return group


def preparation_journey(context: DriverContext) -> None:
    with ControlledServer(context.directory / "http") as server:
        fixture = simple_fixture(
            context.directory / "fixture", server.endpoint, [decision("other")]
        )
        request = OperationRequest(
            name="preview",
            operation="prepare_rules",
            input_value=fixture.input_value,
            cwd=fixture.root,
        )
        preview = run_operation(context, request)
        assert preview["kind"] == "prepared" and server.count == 0
        prepared = object_at(preview, "preparation")
        planned = objects(prepared, "planned_requests")
        assert len(planned) == 2
        assert [object_at(item, "readiness")["kind"] for item in planned] == [
            "prepared",
            "waiting",
        ]
        check_counts(prepared, requests=0, files=1)
        before_size = (fixture.root / "source.md").stat().st_size
        new_subject = "U001: MUTATED_SUBJECT_WITH_DIFFERENT_BYTE_LENGTH\n"
        (fixture.root / "source.md").write_text(new_subject)
        assert len(new_subject.encode()) != before_size
        definition = read_object(fixture.root / "set.json")
        definition["common_ruleset_instructions"] = (
            "MUTATED_COMMON_WITH_DIFFERENT_BYTE_LENGTH"
        )
        write_object(fixture.root / "set.json", definition)
        with context.begin(
            OperationRequest(
                name="fresh-evaluation",
                operation="evaluate_rules",
                input_value=fixture.input_value,
                cwd=fixture.root,
            )
        ) as process:
            gate = server.next()
            assert "MUTATED_SUBJECT" not in gate.raw.decode()
            gate.answer()
            judgment = server.next()
            text = judgment.raw.decode()
            assert "MUTATED_SUBJECT_WITH_DIFFERENT_BYTE_LENGTH" in text
            assert "MUTATED_COMMON_WITH_DIFFERENT_BYTE_LENGTH" in text
            assert "ORIGINAL_SUBJECT" not in text
            judgment.answer()
            report = complete_report(process.finish(0))
        assert len(objects(report, "occurrences")) == 1
        assert objects(report, "occurrences")[0]["kind"] == "judged"
        check_counts(report, requests=2, files=2)
        assert server.count == 2
        # Re-preparation is independent too, and exposes the newly encoded budget.
        # Remove the gate only in a copied definition to make the same rule request ready.
        definition["applicability"] = []
        write_object(fixture.root / "set.json", definition)
        fresh = object_at(
            run_operation(
                context,
                OperationRequest(
                    name="new-accounting",
                    operation="prepare_rules",
                    input_value=fixture.input_value,
                    cwd=fixture.root,
                ),
            ),
            "preparation",
        )
        ready = object_at(objects(fresh, "planned_requests")[0], "readiness")
        counts = object_at(ready, "budget", "counts")
        assert counts["request_bytes"] == len(judgment.raw)
        assert counts["questions"] == len(object_at(judgment.body, "questions"))
        assert isinstance(counts["input_tokens"], int) and counts["input_tokens"] > 0
        assert server.count == 2
        write_object(
            context.directory / "mutation-readback.json",
            {
                "original_source_bytes": before_size,
                "new_source_bytes": len(new_subject.encode()),
                "actual_http_bytes": len(judgment.raw),
                "new_counts": counts,
            },
        )


def grouping_fixture(context: DriverContext, endpoint: str, name: str) -> JsonObject:
    root = context.directory / name
    root.mkdir(parents=True, exist_ok=False)
    for label in ["A", "B"]:
        (root / f"{label}.md").write_text(f"U001: DOCUMENT_{label}\n")
        definition = ruleset(["shared.jsonl"])
        definition["inputs"] = {"subject": {"kind": "document"}}
        definition["common_ruleset_instructions"] = f"OWNER_{label}"
        write_object(root / f"{label}.json", definition)
    write_object(root / "shared.jsonl", noul_rule("shared"))
    invocation = base_invocation(root, endpoint)
    invocation["definitions"] = {
        "kind": "rule_sets",
        "rule_sets": [
            {
                "path": f"{label}.json",
                "binding_groups": [
                    {"bindings": {"subject": document_binding(f"{label}.md")}}
                ],
            }
            for label in ["A", "B"]
        ],
    }
    return invocation


def grouping_journey(context: DriverContext) -> None:
    with ControlledServer(context.directory / "identical-http") as server:
        fixture = simple_fixture(context.directory / "identical", server.endpoint)
        definitions = object_at(fixture.input_value, "definitions")
        owner = as_object(array_at(definitions, "rule_sets")[0])
        groups = array_at(owner, "binding_groups")
        groups.append(copy.deepcopy(groups[0]))
        with context.begin(
            OperationRequest(
                name="identical-groups",
                operation="evaluate_rules",
                input_value=fixture.input_value,
                cwd=fixture.root,
            )
        ) as process:
            held = [server.next(), server.next()]
            for pending in reversed(held):
                pending.answer()
            report = complete_report(process.finish(0))
        occurrences = objects(report, "occurrences")
        assert len(occurrences) == 2 and all(
            item["kind"] == "judged" for item in occurrences
        )
        assert [object_at(item, "key")["group"] for item in occurrences] == [0, 1]
        check_counts(report, requests=2, files=1)
        assert server.count == 2
    with ControlledServer(context.directory / "owners-http") as server:
        invocation = grouping_fixture(context, server.endpoint, "owners")
        root_text = invocation["definition_root"]
        assert isinstance(root_text, str)
        from_root = context.directory / "owners"
        with context.begin(
            OperationRequest(
                name="cross-owner-reverse",
                operation="evaluate_rules",
                input_value=invocation,
                cwd=from_root,
            )
        ) as process:
            held = [server.next(), server.next()]
            classified = [
                (request_owner_label(pending.body), pending) for pending in held
            ]
            assert {label for label, _ in classified} == {"A", "B"}
            for label, pending in reversed(classified):
                pending.answer(0.2 if label == "A" else 0.9)
            report = complete_report(process.finish(0))
        occurrences = objects(report, "occurrences")
        assert len(occurrences) == 2
        assert [object_at(item, "key")["rule_set"] for item in occurrences] == [
            str(from_root / "A.json"),
            str(from_root / "B.json"),
        ]
        assert [
            object_at(item, "judgment", "verdict")["value"] for item in occurrences
        ] == [0.2, 0.9]
        check_counts(report, requests=2, files=2)
        assert server.count == 2
    for limit in ["requests", "files", "bytes"]:
        with ControlledServer(context.directory / f"{limit}-http") as server:
            invocation = grouping_fixture(context, server.endpoint, f"limit-{limit}")
            root = context.directory / f"limit-{limit}"
            object_at(invocation, "execution")["max_concurrent_requests"] = 1
            if limit == "requests":
                object_at(invocation, "request_budget")["max_requests"] = 1
            elif limit == "files":
                object_at(invocation, "limits")["max_files"] = 1
            else:
                one_size = (root / "A.md").stat().st_size
                assert one_size == (root / "B.md").stat().st_size
                object_at(invocation, "limits")["max_total_bytes"] = one_size
            with context.begin(
                OperationRequest(
                    name=f"cumulative-{limit}",
                    operation="evaluate_rules",
                    input_value=invocation,
                    cwd=root,
                )
            ) as process:
                server.next().answer()
                outcome = process.finish(3)
            assert outcome["kind"] == "incomplete_report"
            report = object_at(outcome, "report")
            assert (
                sum(item["kind"] == "judged" for item in objects(report, "occurrences"))
                == 1
            )
            assert len(objects(report, "occurrences")) == 2 and server.count == 1
            assert object_at(report, "measurements")["model_requests"] == 1


def frontier_journey(context: DriverContext) -> None:
    for case in ["nonpass", "failure", "false", "unresolved", "pass", "leading-false"]:
        with ControlledServer(context.directory / f"{case}-http") as server:
            condition: JsonObject = {
                "kind": "condition",
                "test": {
                    "kind": "pattern_present",
                    "input": "other",
                    "pattern": {"kind": "literal", "text": "NOW_TRUE"},
                    "max_scan_bytes": 1 if case == "unresolved" else 4096,
                },
            }
            entries = [decision("subject"), condition, decision("later")]
            if case == "leading-false":
                entries.insert(
                    0,
                    {
                        "kind": "condition",
                        "test": {
                            "kind": "origin_field_equals",
                            "field": "event",
                            "value": "Proceed",
                        },
                    },
                )
            fixture = simple_fixture(context.directory / case, server.endpoint, entries)
            fixture.input_value["origin"] = {
                "kind": "hook",
                "host": "synthetic",
                "event": "Stop",
            }
            (fixture.root / "later.md").write_text("U001: C_ONLY\n")
            definition = read_object(fixture.root / "set.json")
            object_at(definition, "inputs")["later"] = {"kind": "document"}
            write_object(fixture.root / "set.json", definition)
            bindings(fixture.input_value)["later"] = document_binding("later.md")
            preview = run_operation(
                context,
                OperationRequest(
                    name=f"{case}-preview",
                    operation="prepare_rules",
                    input_value=fixture.input_value,
                    cwd=fixture.root,
                ),
            )
            assert preview["kind"] == "prepared" and server.count == 0
            prepared = object_at(preview, "preparation")
            assert object_at(prepared, "measurements")["files_read"] == (
                0 if case == "leading-false" else 1
            )
            if case != "leading-false":
                assert any(
                    object_at(item, "readiness")["kind"] == "waiting"
                    for item in objects(prepared, "planned_requests")
                )
            expected_exit = 3 if case in ["failure", "unresolved"] else 0
            with context.begin(
                OperationRequest(
                    name=f"{case}-evaluate",
                    operation="evaluate_rules",
                    input_value=fixture.input_value,
                    cwd=fixture.root,
                )
            ) as process:
                if case != "leading-false":
                    first = server.next()
                    assert len(object_at(first.body, "questions")) == 1
                    assert (
                        "ORIGINAL_SUBJECT" in first.raw.decode()
                        and "C_ONLY" not in first.raw.decode()
                    )
                    if case in ["pass", "unresolved"]:
                        (fixture.root / "other.md").write_text("U001: NOW_TRUE\n")
                    if case == "failure":
                        first.respond(503, {"error": "controlled failure"})
                    else:
                        first.answer(0.1 if case == "nonpass" else 0.9)
                    if case == "pass":
                        gate = server.next()
                        assert "C_ONLY" in gate.raw.decode()
                        assert (
                            object_at(
                                as_object(
                                    next(
                                        iter(object_at(gate.body, "questions").values())
                                    )
                                ),
                                "instructions",
                            )["owner"]
                            == "rule_set_decision:2"
                        )
                        gate.answer()
                        server.next().answer()
                outcome = process.finish(expected_exit)
            report = object_at(outcome, "report")
            requests = 0 if case == "leading-false" else 3 if case == "pass" else 1
            files = (
                0
                if case == "leading-false"
                else 3
                if case == "pass"
                else 2
                if case in ["false", "unresolved"]
                else 1
            )
            check_counts(report, requests=requests, files=files)
            assert server.count == requests and len(objects(report, "occurrences")) == 1
            occurrence = objects(report, "occurrences")[0]
            assert occurrence["kind"] == (
                "judged"
                if case == "pass"
                else "evaluation_failed"
                if expected_exit == 3
                else "not_applicable"
            )
            if case not in ["pass", "leading-false"]:
                entries_report = objects(objects(report, "groups")[0], "entries")
                assert object_at(entries_report[2], "result")["kind"] == "not_evaluated"


def routing_journey(context: DriverContext) -> None:
    for invalid in [False, True]:
        case = "invalid" if invalid else "opposite-routes"
        with ControlledServer(context.directory / f"{case}-http") as server:
            root = context.directory / case
            root.mkdir(parents=True, exist_ok=False)
            definition = ruleset(["document.jsonl"])
            definition["inputs"] = {"subject": {"kind": "document"}}
            write_object(root / "document.json", definition)
            write_object(root / "document.jsonl", noul_rule("shared"))
            definition["inputs"] = {"subject": {"kind": "conversation"}}
            definition["rule_paths"] = ["conversation.jsonl"]
            write_object(root / "conversation.json", definition)
            write_object(
                root / "conversation.jsonl",
                noul_rule("shared", selection=conversation_selection(tools=False)),
            )
            router: JsonObject = {
                "schema_version": 1,
                "name": "mixed kinds",
                "description": "group isolation",
                "inputs": {
                    "document": {"kind": "document"},
                    "chat": {"kind": "conversation"},
                },
                "evidence": [
                    {
                        "id": "context",
                        "input": "document",
                        "select": {"kind": "whole_document"},
                    }
                ],
                "question": {
                    "type": "choice",
                    "instructions": "select",
                    "criteria": {"document": "doc", "conversation": "chat"},
                },
                "routes": {
                    "document": [
                        {"rule_set": "document.json", "inputs": {"subject": "document"}}
                    ],
                    "conversation": [
                        {
                            "rule_set": "conversation.json",
                            "inputs": {"subject": "document" if invalid else "chat"},
                        }
                    ],
                },
                "confidence_floor": 0.6,
            }
            write_object(root / "router.json", router)
            groups: list[JsonObject] = []
            for group in [0, 1]:
                (root / f"doc-{group}.md").write_text(f"U001: DOC_GROUP_{group}\n")
                write_object(
                    root / f"chat-{group}.jsonl",
                    {
                        "type": "user",
                        "uuid": f"u{group}",
                        "message": {"role": "user", "content": f"CHAT_GROUP_{group}"},
                    },
                )
                groups.append(
                    {
                        "bindings": {
                            "document": document_binding(f"doc-{group}.md"),
                            "chat": {
                                "kind": "conversation",
                                "transcript_path": str(root / f"chat-{group}.jsonl"),
                                "format": "claude_code_jsonl",
                            },
                        }
                    }
                )
            invocation = base_invocation(root, server.endpoint)
            invocation["definitions"] = {
                "kind": "router",
                "path": "router.json",
                "binding_groups": groups,
            }
            with context.begin(
                OperationRequest(
                    name=case,
                    operation="evaluate_rules",
                    input_value=invocation,
                    cwd=root,
                )
            ) as process:
                if invalid:
                    outcome = process.finish(2)
                    assert outcome[
                        "kind"
                    ] == "invalid" and "invalid_wiring" in json.dumps(outcome)
                    assert server.count == 0
                    continue
                held = [server.next(), server.next()]
                classified = [
                    (request_router_group(pending.body), pending) for pending in held
                ]
                assert {group for group, _ in classified} == {0, 1}
                for group, pending in reversed(classified):
                    pending.answer(choice="document" if group == 0 else "conversation")
                chat = server.next()
                assert "CHAT_GROUP_1" in chat.raw.decode()
                assert (
                    "CHAT_GROUP_0" not in chat.raw.decode()
                    and "DOC_GROUP" not in chat.raw.decode()
                )
                chat.answer()
                report = complete_report(process.finish(0))
            assert server.count == 3
            occurrences = objects(report, "occurrences")
            assert len(occurrences) == 2 and all(
                item["kind"] == "judged" for item in occurrences
            )
            keys = [object_at(item, "key") for item in occurrences]
            assert {(item["rule_set"], item["group"]) for item in keys} == {
                (str(root / "document.json"), 0),
                (str(root / "conversation.json"), 1),
            }
            routed_groups = objects(report, "groups")
            assert {
                json.dumps(object_at(item, "binding"), sort_keys=True)
                for item in routed_groups
            } == {
                json.dumps(
                    {"kind": "routed", "router_group": group, "label": label},
                    sort_keys=True,
                )
                for group, label in [(0, "document"), (1, "conversation")]
            }
            for occurrence in occurrences:
                target = object_at(occurrence, "judgment", "target")
                if object_at(occurrence, "key")["group"] == 0:
                    assert target["kind"] == "paragraph"
                    assert object_at(target, "span", "source")["path"] == "doc-0.md"
                else:
                    assert target["kind"] == "conversation_records"
                    assert object_at(target, "source")["path"] == str(
                        root / "chat-1.jsonl"
                    )
                    assert target["record_ids"] == ["u1"]
            routes = objects(object_at(report, "routing"), "groups")
            references = [object_at(item, "outcome", "answer") for item in routes]
            assert references[0] != references[1]
            assert [
                native_answer(report, reference)["selected_label"]
                for reference in references
            ] == ["document", "conversation"]
