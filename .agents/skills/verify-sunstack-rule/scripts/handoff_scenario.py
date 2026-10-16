"""Reviewed handoff definitions with an owned, qualified Claude composition."""

import hashlib
import json
import os
import re
import subprocess

from composition_scenarios import native_answer
from scenario_fixtures import conversation_selection, evidence_plan
from scenario_runtime import (
    JSON_OBJECT,
    REPOSITORY_ROOT,
    SKILL_ROOT,
    DriverContext,
    JsonObject,
    OperationRequest,
    array_at,
    as_object,
    complete_report,
    object_at,
    read_object,
    run_operation,
    write_object,
)
from scenario_transport import ControlledServer

EXAMPLES = "docs/specs/2026-10-12-rust-rule-engine/examples"


def reviewed_fixture(context: DriverContext, endpoint: str, case: str) -> JsonObject:
    root = context.directory / case
    root.mkdir(parents=True, exist_ok=False)
    pin = re.search(
        r"design revision `([0-9a-f]{40})`", (SKILL_ROOT / "SKILL.md").read_text()
    )
    assert pin, "Read the oracle revision from its one owner"
    design = pin.group(1)
    names = [
        "agent-handoff.router.json",
        "review-handoff.ruleset.json",
        "review-handoff-rules.jsonl",
        "commission-completeness.ruleset.json",
        "commission-rules.jsonl",
        "handoff-monitor.invocation.json",
    ]
    for name in names:
        value = subprocess.check_output(
            ["git", "show", f"{design}:{EXAMPLES}/{name}"], cwd=REPOSITORY_ROOT
        )
        (root / name).write_bytes(value)
    source = SKILL_ROOT / "scripts/fixtures/usage-scenarios/claude-handoff.jsonl"
    transcript = root / "claude.jsonl"
    transcript.write_bytes(source.read_bytes())
    invocation = read_object(root / "handoff-monitor.invocation.json")
    invocation["definition_root"] = str(root)
    definitions = object_at(invocation, "definitions")
    group = as_object(array_at(definitions, "binding_groups")[0])
    session = object_at(group, "bindings", "session")
    session["transcript_path"] = str(transcript)
    session["format"] = "claude_code_jsonl"
    del session["final_assistant_text"]
    records = [read_object_line(line) for line in source.read_text().splitlines()]
    final = records[-1]
    final_text = "\n".join(
        str(as_object(block)["text"]) for block in array_at(final, "message", "content")
    )
    if case == "supplement":
        transcript.write_text(
            "\n".join(json.dumps(record) for record in records[:-1]) + "\n"
        )
        session["final_assistant_text"] = final_text
    origin = object_at(invocation, "origin")
    origin["host"] = "claude"
    origin["event"] = "SubagentStop" if case == "subagent" else "Stop"
    origin["agent"] = {
        "kind": "subagent",
        "id": "synthetic-reviewer",
        "type": "reviewer",
    }
    invocation["payload_schemas"] = [
        {
            "name": "agent-job-packet",
            "path": str(
                REPOSITORY_ROOT
                / "crates/sunstack-evidence/tests/fixtures/agent-job-packet.schema.json"
            ),
        }
    ]
    object_at(invocation, "backend")["endpoint"] = endpoint
    object_at(invocation, "attempts")["retry_count"] = 0
    object_at(invocation, "attempts")["attempt_timeout_ms"] = 60000
    object_at(invocation, "request_budget")["max_requests"] = 1
    write_object(
        root / "adaptation.json",
        {
            "design_revision": design,
            "source_paths": [f"{EXAMPLES}/{name}" for name in names],
            "deltas": [
                "definition root and owned Claude transcript binding",
                "declared generated schema path",
                "supplied Claude hook provenance",
                "endpoint and fixed 60000ms/zero-retry/one-request settings",
            ],
            "fixture_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
            "final_text": final_text,
        },
    )
    write_object(root / "invocation.json", invocation)
    return invocation


def read_object_line(line: str) -> JsonObject:
    return JSON_OBJECT.validate_json(line)


def check_handoff(report: JsonObject, invocation: JsonObject) -> None:
    routing = as_object(array_at(report, "routing", "groups")[0])
    assert object_at(routing, "outcome")["kind"] == "routed"
    assert object_at(routing, "outcome")["label"] == "review_handback"
    assert (
        native_answer(report, object_at(routing, "outcome", "answer"))["selected_label"]
        == "review_handback"
    )
    occurrences = [as_object(item) for item in array_at(report, "occurrences")]
    assert [object_at(item, "key")["rule_id"] for item in occurrences] == [
        "reviewer.read-required-sources",
        "reviewer.stayed-read-only",
        "reviewer.handback-addresses-target",
    ]
    assert all(item["kind"] == "judged" for item in occurrences)
    assert [
        object_at(item, "judgment")["declared_severity"] for item in occurrences
    ] == ["block", "block", "nudge"]
    supplied = object_at(invocation, "origin")
    assert object_at(report, "origin")["host"] == supplied["host"]
    assert object_at(report, "origin")["event"] == supplied["event"]
    for field in ["session", "turn", "observed_at"]:
        assert object_at(report, "origin", field) == {
            "kind": "observed",
            "value": supplied[field],
        }
    assert object_at(report, "origin", "agent", "type")["value"] == "reviewer"
    session = object_at(
        as_object(array_at(invocation, "definitions", "binding_groups")[0]),
        "bindings",
        "session",
    )
    expected_ids = [["tool:read-spec", "tool:read-design"], ["tool:read-design"]]
    for position, occurrence in enumerate(occurrences):
        target = object_at(occurrence, "judgment", "target")
        assert target["kind"] == "conversation_records"
        assert object_at(target, "source")["format"] == "claude_code_jsonl"
        assert object_at(target, "source")["path"] == session["transcript_path"]
        if position < 2:
            assert target["record_ids"] == expected_ids[position]
        else:
            record_ids = array_at(target, "record_ids")
            assert len(record_ids) == 1
            record_id = record_ids[0]
            assert isinstance(record_id, str)
            assert record_id == "review-handback" or record_id.startswith("hook:")
        for reference in object_at(occurrence, "judgment", "answers").values():
            assert native_answer(report, as_object(reference))
        basis = object_at(occurrence, "judgment", "verdict", "basis")
        if basis["kind"] == "noul":
            assert native_answer(report, object_at(basis, "answer"))["type"] == "noul"
    assert len(array_at(report, "cursors")) == 1


def handoff_journey(context: DriverContext) -> None:
    for case in ["main", "subagent", "supplement"]:
        with ControlledServer(context.directory / f"{case}-http") as server:
            invocation = reviewed_fixture(context, server.endpoint, case)
            root = context.directory / case
            bound = object_at(
                as_object(array_at(invocation, "definitions", "binding_groups")[0]),
                "bindings",
                "session",
            )
            plan = evidence_plan(
                root,
                {"session": bound},
                [
                    {
                        "id": "all",
                        "input": "session",
                        "requirement": "required",
                        "select": conversation_selection(),
                    }
                ],
            )
            prepared = run_operation(
                context,
                OperationRequest(
                    name=f"{case}-whole-records",
                    operation="prepare_evidence",
                    input_value=plan,
                    cwd=root,
                    credential="none",
                ),
            )
            records = [
                as_object(record)
                for record in array_at(
                    as_object(array_at(prepared, "selections")[0]),
                    "readiness",
                    "content",
                    "records",
                )
            ]
            tools = [
                record
                for record in records
                if object_at(record, "body")["kind"] == "tool_use"
            ]
            assert [object_at(record, "body")["call_id"] for record in tools] == [
                "read-spec",
                "read-design",
            ]
            assert all(
                object_at(record, "body")["outcome"] == "succeeded" for record in tools
            )
            assert all(
                object_at(record, "body", "result")["kind"] == "full_observed"
                for record in tools
            )
            final = records[-1]
            assert final["provenance"] == (
                "hook_supplement" if case == "supplement" else "transcript"
            )
            assert final["injection"] == "not_injected"
            blocks = array_at(final, "body", "blocks")
            assert "Finding one" in json.dumps(blocks) and "Finding two" in json.dumps(
                blocks
            )
            assert len(blocks) == (1 if case == "supplement" else 2)
            with context.begin(
                OperationRequest(
                    name=f"{case}-rules",
                    operation="evaluate_rules",
                    input_value=invocation,
                    cwd=root,
                )
            ) as process:
                pending = server.next()
                assert (
                    "Finding one" in pending.raw.decode()
                    and "Finding two" in pending.raw.decode()
                )
                pending.answer()
                report = complete_report(process.finish(0))
            check_handoff(report, invocation)
            assert server.count == 1
            assert object_at(report, "measurements")["model_requests"] == 1
            if case == "main":
                frozen = read_object(root / "invocation.json")
                object_at(frozen, "backend")["endpoint"] = (
                    "https://openrouter.ai/api/v1/systemone"
                )
                write_object(context.directory / "hosted-frozen.json", frozen)


def hosted_handoff(context: DriverContext) -> None:
    assert (context.directory / "local-pass.json").is_file(), (
        "Local US02 controls must pass first"
    )
    slot = context.directory / "hosted-slot.json"
    assert not slot.exists(), "The genuine US02 slot cannot be retried"
    invocation = read_object(context.directory / "hosted-frozen.json")
    local = read_object(context.directory / "local-pass.json")
    assert local["product_commit"] == context.product_commit
    assert (
        local["binary_sha256"]
        == hashlib.sha256(context.binary.read_bytes()).hexdigest()
    )
    assert os.environ.get("OPENROUTER_API_KEY"), (
        "Credential prerequisite missing; no slot spent"
    )
    original = read_object(context.directory / "02-main-rules/stdin.json")
    expected = object_at(original, "input")
    object_at(expected, "backend")["endpoint"] = (
        "https://openrouter.ai/api/v1/systemone"
    )
    assert invocation == expected, (
        "Frozen invocation changed after its controlled positive"
    )
    adaptation = read_object(context.directory / "main/adaptation.json")
    revision = adaptation["design_revision"]
    assert isinstance(revision, str)
    for source_path in array_at(adaptation, "source_paths"):
        assert isinstance(source_path, str)
        original_bytes = subprocess.check_output(
            ["git", "show", f"{revision}:{source_path}"], cwd=REPOSITORY_ROOT
        )
        assert (
            context.directory / "main" / source_path.rsplit("/", 1)[1]
        ).read_bytes() == original_bytes
    assert (context.directory / "main/claude.jsonl").read_bytes() == (
        SKILL_ROOT / "scripts/fixtures/usage-scenarios/claude-handoff.jsonl"
    ).read_bytes()
    preflight = context.directory / "signing-preflight.log"
    commands = [
        [
            str(REPOSITORY_ROOT / "scripts/cargo-debug-signing-runner.sh"),
            "--sign-only",
            str(context.binary),
        ],
        ["codesign", "-dv", str(context.binary)],
        ["codesign", "--verify", str(context.binary)],
    ]
    with preflight.open("wb") as stream:
        for command in commands:
            result = subprocess.run(
                command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False
            )
            stream.write(result.stdout)
            assert result.returncode == 0, "Signing preflight failed before dispatch"
    signature = preflight.read_text()
    assert "runs unsigned" not in signature
    assert "Identifier=dev.shravansunder.sunstack-rule.debug" in signature
    assert "TeamIdentifier=974QD84WVC" in signature
    # Exclusive durable custody precedes dispatch; any later failure consumes it.
    with slot.open("x") as stream:
        stream.write(
            '{"journey":"US02","state":"dispatch-authorized-once","max_requests":1,"retry_count":0}\n'
        )
    with context.begin(
        OperationRequest(
            name="genuine-once",
            operation="evaluate_rules",
            input_value=invocation,
            cwd=context.directory / "main",
            credential="hosted",
        )
    ) as process:
        report = complete_report(process.finish(0))
    check_handoff(report, invocation)
    sent = [
        as_object(item)
        for item in array_at(report, "requests")
        if object_at(as_object(item), "status")["kind"] == "sent"
    ]
    assert len(sent) == 1
    result = object_at(sent[0], "status", "outcome")
    assert result["kind"] == "completed" and len(array_at(result, "attempts")) == 1
    attempt = object_at(as_object(array_at(result, "attempts")[0]), "evidence")
    assert attempt["kind"] == "completed" and attempt["http_status"] == 200
    receipt = object_at(result, "receipt")
    assert isinstance(receipt["resolved_model"], str) and receipt["resolved_model"]
    tokens = object_at(receipt, "usage")["input_tokens"]
    assert isinstance(tokens, int) and tokens > 0
    write_object(
        context.directory / "hosted-readback.json",
        {
            "receipt": receipt,
            "attempts": result["attempts"],
            "routing": report["routing"],
            "occurrences": report["occurrences"],
        },
    )
