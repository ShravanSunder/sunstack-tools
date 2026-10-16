"""US15: serialize the caller's cursor through fresh executable processes."""

import json
from dataclasses import dataclass
from pathlib import Path

from scenario_fixtures import conversation_selection, evidence_plan
from scenario_runtime import (
    SKILL_ROOT,
    DriverContext,
    JsonObject,
    OperationRequest,
    array_at,
    as_array,
    as_object,
    object_at,
    read_object,
    run_operation,
)


def line(record: JsonObject) -> bytes:
    return (json.dumps(record) + "\n").encode()


def assistant_record(format_name: str, record_id: str, text: str) -> JsonObject:
    if format_name == "claude_code_jsonl":
        return {
            "type": "assistant",
            "message": {"id": record_id, "content": [{"type": "text", "text": text}]},
        }
    return {
        "type": "response_item",
        "payload": {
            "type": "message",
            "role": "assistant",
            "id": record_id,
            "content": [{"type": "output_text", "text": text}],
        },
    }


def result_record(format_name: str) -> JsonObject:
    if format_name == "claude_code_jsonl":
        return {
            "type": "user",
            "message": {
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "read1",
                        "content": "OBSERVED_READ_RESULT",
                        "is_error": False,
                    }
                ]
            },
        }
    return {
        "type": "response_item",
        "payload": {
            "type": "function_call_output",
            "call_id": "read1",
            "output": "OBSERVED_READ_RESULT",
        },
    }


@dataclass(kw_only=True)
class CursorSession:
    context: DriverContext
    path: Path
    format_name: str
    input_value: JsonObject

    def append(self, record: JsonObject) -> None:
        with self.path.open("ab") as stream:
            stream.write(line(record))

    def capture(self, name: str, expected_exit: int = 0) -> JsonObject:
        outcome = run_operation(
            self.context,
            OperationRequest(
                name=name,
                operation="prepare_evidence",
                input_value=self.input_value,
                cwd=self.path.parent,
                credential="none",
                expected_exit=expected_exit,
            ),
        )
        if expected_exit == 0:
            assert outcome["kind"] == "prepared"
            self.input_value["cursors"] = outcome["cursors"]
            capture = as_object(array_at(outcome, "captures")[0])
            assert capture["bytes_read"] == self.path.stat().st_size
            parsed = capture["bytes_parsed"]
            assert isinstance(parsed, int) and parsed <= self.path.stat().st_size
        return outcome


def records(outcome: JsonObject) -> list[JsonObject]:
    selection = as_object(array_at(outcome, "selections")[0])
    return [
        as_object(value)
        for value in array_at(selection, "readiness", "content", "records")
    ]


def make_session(context: DriverContext, format_name: str, name: str) -> CursorSession:
    directory = context.directory / "fixtures" / f"{format_name}-{name}"
    directory.mkdir(parents=True, exist_ok=False)
    seeds = read_object(
        SKILL_ROOT / "scripts/fixtures/usage-scenarios/cursor-seeds.json"
    )
    seed_records = [as_object(value) for value in as_array(seeds[format_name])]
    path = directory / "transcript.jsonl"
    path.write_bytes(b"".join(line(record) for record in seed_records))
    plan = evidence_plan(
        directory,
        {
            "session": {
                "kind": "conversation",
                "transcript_path": str(path),
                "format": format_name,
            }
        },
        [
            {
                "id": "selected",
                "input": "session",
                "requirement": "required",
                "select": conversation_selection("since_cursor"),
            }
        ],
    )
    return CursorSession(
        context=context, path=path, format_name=format_name, input_value=plan
    )


def cursor_journey(context: DriverContext) -> None:
    for format_name in ["claude_code_jsonl", "codex_rollout_jsonl"]:
        session = make_session(context, format_name, "timeline")
        prefix = "claude" if format_name == "claude_code_jsonl" else "codex"
        initial = session.capture(f"{prefix}-initial")
        assert len(records(initial)) == 3
        assert (
            as_object(array_at(initial, "captures")[0])["cursor_basis"]
            == "no_cursor_supplied"
        )
        pending = next(
            record
            for record in records(initial)
            if object_at(record, "body")["kind"] == "tool_use"
        )
        assert object_at(pending, "body")["call_id"] == "read1"
        assert object_at(pending, "body")["outcome"] == "result_missing"
        if format_name == "claude_code_jsonl":
            session.append(assistant_record(format_name, "a1", "STREAMED_NEW_BLOCK"))
            streamed = records(session.capture("claude-streamed-same-id"))
            assert len(streamed) == 1 and streamed[0]["record_id"] == "a1"
            blocks = array_at(streamed[0], "body", "blocks")
            assert len(blocks) == 3
            assert as_object(blocks[0])["text"] == "FIRST_BLOCK"
            assert as_object(blocks[2])["text"] == "STREAMED_NEW_BLOCK"
        late = line(result_record(format_name))
        split = len(late) // 2
        with session.path.open("ab") as stream:
            stream.write(late[:split])
        partial = session.capture(f"{prefix}-partial-tail")
        assert records(partial) == []
        assert (
            object_at(as_object(array_at(partial, "captures")[0]), "tail")["kind"]
            == "incomplete"
        )
        with session.path.open("ab") as stream:
            stream.write(late[split:])
        observed = records(session.capture(f"{prefix}-late-result"))
        assert len(observed) == 1
        tool = object_at(observed[0], "body")
        assert tool["kind"] == "tool_use" and tool["call_id"] == "read1"
        assert tool["outcome"] == (
            "succeeded" if prefix == "claude" else "result_present_status_unknown"
        )
        assert object_at(tool, "result") == {
            "kind": "full_observed",
            "text": "OBSERVED_READ_RESULT",
        }
        assert records(session.capture(f"{prefix}-unchanged-result")) == []
        if prefix == "claude":
            user: JsonObject = {
                "type": "user",
                "origin": {"kind": "user"},
                "message": {"id": "u2", "content": "Finish the review."},
            }
        else:
            user = {
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": "Finish the review."}],
                },
            }
        session.append(user)
        session.append(assistant_record(format_name, "a2", "SECOND_TURN"))
        assert len(records(session.capture(f"{prefix}-second-turn"))) == 2
        source = object_at(session.input_value, "sources", "session")
        source["final_assistant_text"] = "FIRST_SUPPLEMENT"
        first = records(session.capture(f"{prefix}-first-supplement"))
        assert len(first) == 1 and first[0]["provenance"] == "hook_supplement"
        assert first[0]["injection"] == "not_injected"
        assert records(session.capture(f"{prefix}-unchanged-supplement")) == []
        source["final_assistant_text"] = "CHANGED_WHOLE_SUPPLEMENT"
        changed = records(session.capture(f"{prefix}-changed-supplement"))
        assert len(changed) == 1
        assert array_at(changed[0], "body", "blocks") == [
            {"kind": "text", "text": "CHANGED_WHOLE_SUPPLEMENT"}
        ]
        del source["final_assistant_text"]
        session.append(
            assistant_record(
                format_name, "materialized-final", "CHANGED_WHOLE_SUPPLEMENT"
            )
        )
        materialized = records(session.capture(f"{prefix}-materialized-final"))
        assert len(materialized) == 1 and materialized[0]["provenance"] == "transcript"
        assert materialized[0]["injection"] == "not_injected"
        assert records(session.capture(f"{prefix}-unchanged-materialized")) == []
        for mutation in ["rewrite", "replacement"]:
            branch = make_session(context, format_name, mutation)
            branch.capture(f"{prefix}-{mutation}-valid-cursor")
            original = branch.path.read_bytes()
            if mutation == "rewrite":
                rewritten = original.replace(b"FIRST_BLOCK", b"OTHER_BLOCK", 1)
                assert len(rewritten) == len(original) and rewritten != original
                branch.path.write_bytes(rewritten)
            else:
                replacement = branch.path.with_name("replacement.jsonl")
                replacement.write_bytes(original)
                replacement.replace(branch.path)
            refused = branch.capture(f"{prefix}-{mutation}-refused", 3)
            assert refused["kind"] == "incomplete"
            assert (
                as_object(array_at(refused, "diagnostics", "items")[0])["code"]
                == "transcript_discontinuous"
            )
