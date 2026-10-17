"""Hook provenance and schema-admitted tool payloads through the real CLI."""

import copy
import json

from scenario_fixtures import bindings, noul_rule, simple_fixture
from scenario_runtime import (
    SKILL_ROOT,
    DriverContext,
    JsonObject,
    OperationRequest,
    array_at,
    as_object,
    complete_report,
    object_at,
    read_object,
    write_object,
)
from scenario_transport import ControlledServer


def origin_journey(context: DriverContext) -> None:
    for case in ["sparse", "direct-condition", "missing-agent", "populated-agent"]:
        with ControlledServer(context.directory / f"{case}-http") as server:
            entries: list[JsonObject] = []
            if case != "sparse":
                entries = [
                    {
                        "kind": "condition",
                        "test": {
                            "kind": "origin_field_equals",
                            "field": "event"
                            if case == "direct-condition"
                            else "agent_type",
                            "value": "Stop"
                            if case == "direct-condition"
                            else "reviewer",
                        },
                    }
                ]
            fixture = simple_fixture(context.directory / case, server.endpoint, entries)
            if case != "direct-condition":
                fixture.input_value["origin"] = {
                    "kind": "hook",
                    "host": "claude",
                    "event": "Stop",
                }
            if case == "populated-agent":
                object_at(fixture.input_value, "origin")["agent"] = {
                    "kind": "subagent",
                    "id": "reviewer-owned",
                    "type": "reviewer",
                }
            passes = case in ["sparse", "populated-agent"]
            with context.begin(
                OperationRequest(
                    name=case,
                    operation="evaluate_rules",
                    input_value=fixture.input_value,
                    cwd=fixture.root,
                )
            ) as process:
                if passes:
                    server.next().answer()
                outcome = process.finish(0 if passes else 3)
            report = object_at(outcome, "report")
            assert len(array_at(report, "occurrences")) == 1
            occurrence = as_object(array_at(report, "occurrences")[0])
            assert occurrence["kind"] == ("judged" if passes else "evaluation_failed")
            assert object_at(report, "measurements")["files_read"] == int(passes)
            assert server.count == int(passes)
            origin = object_at(report, "origin")
            if case == "sparse":
                for field in ["session", "turn", "observed_at"]:
                    assert object_at(origin, field)["kind"] == "unavailable"
                assert object_at(origin, "agent")["kind"] == "unavailable"
            elif case == "populated-agent":
                assert object_at(origin, "agent", "type")["value"] == "reviewer"
            else:
                group = as_object(array_at(report, "groups")[0])
                assert (
                    object_at(group, "applicability")["kind"] == "applicability_failed"
                )
                assert "unresolved" in json.dumps(group)


def payload_journey(context: DriverContext) -> None:
    payloads: dict[str, JsonObject] = {
        "proposal": {
            "kind": "proposal",
            "arguments": {"command": "read public source"},
        },
        "observation": {
            "kind": "observation",
            "arguments": {"command": "read public source"},
            "outcome": {"status": "succeeded", "text": "OBSERVED_TOOL_RESULT"},
        },
    }
    for case in ["proposal", "observation", "wrong-kind", "schema-mismatch"]:
        with ControlledServer(context.directory / f"{case}-http") as server:
            fixture = simple_fixture(context.directory / case, server.endpoint)
            definition = read_object(fixture.root / "set.json")
            definition["inputs"] = {
                "subject": {"kind": "payload", "schema": "tool-event"}
            }
            kind = case if case in payloads else "proposal"
            definition["applicability"] = [
                {
                    "kind": "condition",
                    "test": {
                        "kind": "payload_field_equals",
                        "input": "subject",
                        "pointer": "/kind",
                        "value": kind,
                    },
                }
            ]
            write_object(fixture.root / "set.json", definition)
            write_object(
                fixture.root / "rules.jsonl",
                noul_rule(selection={"kind": "whole_payload"}),
            )
            schema = (
                SKILL_ROOT / "scripts/fixtures/usage-scenarios/tool-payload.schema.json"
            )
            fixture.input_value["payload_schemas"] = [
                {"name": "tool-event", "path": str(schema)}
            ]
            value = copy.deepcopy(payloads[kind])
            if case == "schema-mismatch":
                value["outcome"] = {"status": "invented-result-in-proposal"}
            if case != "wrong-kind":
                group = bindings(fixture.input_value)
                group.clear()
                group["subject"] = {"kind": "payload", "value": value}
            valid = case in payloads
            with context.begin(
                OperationRequest(
                    name=case,
                    operation="evaluate_rules",
                    input_value=fixture.input_value,
                    cwd=fixture.root,
                )
            ) as process:
                if valid:
                    pending = server.next()
                    text = pending.raw.decode()
                    assert "read public source" in text
                    if case == "proposal":
                        assert "OBSERVED_TOOL_RESULT" not in text
                        assert '"outcome"' not in text
                    else:
                        assert "OBSERVED_TOOL_RESULT" in text and "succeeded" in text
                    pending.answer()
                outcome = process.finish(0 if valid else 2)
            if valid:
                report = complete_report(outcome)
                occurrence = as_object(array_at(report, "occurrences")[0])
                assert occurrence["kind"] == "judged"
                assert object_at(occurrence, "judgment", "target") == {
                    "kind": "payload_field",
                    "input": "subject",
                    "pointer": "",
                }
            else:
                assert outcome["kind"] == "invalid"
            assert server.count == int(valid)
