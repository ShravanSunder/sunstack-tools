# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic==2.14.1", "pydantic_core==2.50.1", "ruff==0.17.1", "basedpyright==1.40.2"]
# ///
"""Standalone helper checks and exact pinned Python quality gates."""

import argparse
import copy
import importlib.metadata
import json
import os
import subprocess
import sys
import threading
import urllib.request
from functools import partial
from pathlib import Path
from queue import Queue

from composition_scenarios import request_owner_label, request_router_group
from connected_scenarios import connected_checks
from git_scenario_fixtures import fixture_git_checks
from handoff_scenario import (
    check_final_record,
    check_handoff_request,
    expected_final_blocks,
)
from pydantic import ValidationError
from scenario_readback_checks import (
    assertion_rejects,
    correction_checks,
    readback_checks,
    source_checks,
    window_checks,
)
from scenario_runtime import (
    INDEX_VARIABLES,
    REPOSITORY_ROOT,
    SKILL_ROOT,
    JsonObject,
    OperationFrame,
    object_at,
    read_product_pin,
    write_object,
)
from scenario_transport import ControlledServer, response_body


def sensitivity_checks(evidence: Path) -> None:
    outcomes: JsonObject = {}
    for supplement in [False, True]:
        expected = expected_final_blocks()
        blocks = (
            [
                {
                    "kind": "text",
                    "text": "\n".join(str(item["text"]) for item in expected),
                }
            ]
            if supplement
            else expected
        )
        record: JsonObject = {
            "record_id": "hook:synthetic" if supplement else "review-handback",
            "provenance": "hook_supplement" if supplement else "transcript",
            "injection": "not_injected",
            "body": {"kind": "assistant_message", "blocks": blocks},
        }
        check_final_record(record, supplement)
        truncated = copy.deepcopy(record)
        object_at(truncated, "body")["blocks"] = (
            [{"kind": "text", "text": "Finding one\nFinding two"}]
            if supplement
            else [
                {"kind": "text", "text": "Finding one"},
                {"kind": "text", "text": "Finding two"},
            ]
        )
        outcomes[
            f"us02-{'supplement' if supplement else 'transcript'}-truncated-record"
        ] = assertion_rejects(partial(check_final_record, truncated, supplement))
        owners = [
            "router/recent_turns",
            "rule:reviewer.handback-addresses-target/handback",
            "rule:reviewer.handback-addresses-target:decision:0/final_message",
        ]
        request: JsonObject = {
            "state": {
                "contents": {"final": [record]},
                "evidence": {owner: {"content": "final"} for owner in owners},
            }
        }
        check_handoff_request(request, supplement)
        object_at(request, "state", "contents")["final"] = [truncated]
        outcomes[
            f"us02-{'supplement' if supplement else 'transcript'}-truncated-http"
        ] = assertion_rejects(partial(check_handoff_request, request, supplement))
    owner: JsonObject = {
        "questions": {
            "q1": {"instructions": {"common_ruleset_instructions": "OWNER_B"}}
        },
        "state": {
            "contents": {"c1": "U001: DOCUMENT_B"},
            "evidence": {
                "rule:shared/target": {
                    "content": "c1",
                    "kind": "paragraph",
                    "input": "subject",
                }
            },
        },
    }
    assert request_owner_label(owner) == "B"
    object_at(owner, "questions", "q1", "instructions").pop(
        "common_ruleset_instructions"
    )
    outcomes["us09-missing-owner-b-instructions"] = assertion_rejects(
        lambda: request_owner_label(owner)
    )
    router: JsonObject = {
        "questions": {
            "q1": {"instructions": {"owner": "router", "reads": ["router/context"]}}
        },
        "state": {
            "contents": {"c1": "U001: DOC_GROUP_1\n", "c2": "U001: DOC_GROUP_1"},
            "evidence": {
                "router/context": {
                    "content": "c1",
                    "kind": "document_text",
                    "input": "document",
                }
            },
        },
    }
    assert request_router_group(router) == 1
    object_at(router, "state", "contents")["c1"] = ""
    outcomes["us12-missing-router-group-1-document"] = assertion_rejects(
        lambda: request_router_group(router)
    )
    write_object(evidence / "sensitivity-checks.json", outcomes)
    assert all(value is True for value in outcomes.values()), (
        "A malformed observation was admitted"
    )


def pin_reader_checks() -> None:
    read_product_pin((SKILL_ROOT / "SKILL.md").read_text())
    pin = "a" * 40
    token = f"Product target: `{pin}`"
    assert read_product_pin(f"Branch-independent sentence. {token}.") == pin
    refused = 0
    for invalid in [
        "No token",
        f"{token}\n{token}",
        "Product target: `short`",
        f"{token}\nProduct target: `short`",
    ]:
        try:
            read_product_pin(invalid)
        except ValueError:
            refused += 1
    assert refused == 4


def helper_checks(evidence: Path) -> None:
    frame = {
        "kind": "operation_result",
        "protocol_version": 1,
        "result": {"operation": "prepare_evidence", "outcome": {"kind": "prepared"}},
    }
    assert OperationFrame.model_validate(frame).result.outcome["kind"] == "prepared"
    refused = 0
    for invalid in [
        {**frame, "protocol_version": 2},
        {**frame, "extra": True},
        {"kind": "protocol_error"},
    ]:
        try:
            OperationFrame.model_validate(invalid)
        except ValidationError:
            refused += 1
    assert refused == 3
    body: JsonObject = {
        "questions": {
            "n": {"type": "noul"},
            "c": {"type": "choice", "criteria": {"none": "none"}},
            "s": {"type": "score", "criteria": ["low", "high"]},
        }
    }
    response = response_body(body, 0.8, "none")
    assert response["answers"] == {
        "n": {"type": "noul", "noul": 0.8},
        "c": {
            "type": "choice",
            "choice": "none",
            "confidence": 1.0,
            "probabilities": {"none": 1.0},
        },
        "s": {
            "type": "score",
            "score": 1.0,
            "confidence": 1.0,
            "legend": {"0": "low", "1": "high"},
            "probabilities": {"0": 0.0, "1": 1.0},
        },
    }
    results: Queue[bytes | OSError] = Queue()
    with ControlledServer(evidence / "helper-http") as server:

        def request() -> None:
            try:
                message = urllib.request.Request(
                    server.endpoint,
                    data=json.dumps(body).encode(),
                    headers={"Content-Type": "application/json"},
                    method="POST",
                )
                with urllib.request.urlopen(message, timeout=5) as reply:
                    results.put(reply.read())
            except OSError as failure:
                results.put(failure)

        child = threading.Thread(target=request)
        child.start()
        try:
            pending = server.next()
            assert pending.body == body and pending.raw == json.dumps(body).encode()
            pending.answer(0.8, choice="none")
            duplicate_refused = False
            try:
                pending.answer()
            except AssertionError:
                duplicate_refused = True
            assert duplicate_refused
        finally:
            child.join(timeout=6)
        assert not child.is_alive()
        result = results.get(timeout=1)
        if isinstance(result, OSError):
            raise result
        assert json.loads(result) == response and server.count == 1
    write_object(
        evidence / "helper-checks.json",
        {
            "frame_controls": 4,
            "http_requests": 1,
            "duplicate_response_refused": duplicate_refused,
            "client_thread_alive_after_join": child.is_alive(),
        },
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, required=True)
    arguments = parser.parse_args()
    assert not any(os.environ.get(name) for name in INDEX_VARIABLES)
    evidence = Path(arguments.evidence).resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    assert Path(os.environ["UV_CACHE_DIR"]).resolve().is_relative_to(evidence)
    fixture_git_checks(evidence)
    connected_checks(evidence)
    readback_checks(evidence)
    correction_checks(evidence)
    window_checks(evidence)
    source_checks(evidence)
    rejection_environment = os.environ.copy()
    rejection_environment.pop("OPENROUTER_API_KEY", None)
    refusals: list[JsonObject] = []
    for journey in ["US02", "US21"]:
        refused_evidence = evidence / f"spent-{journey}"
        rejected = subprocess.run(
            [
                sys.executable,
                str(SKILL_ROOT / "scripts/usage_scenarios.py"),
                "--binary",
                str(evidence / "not-executed"),
                "--evidence",
                str(refused_evidence),
                "--journey",
                journey,
                "--hosted",
            ],
            cwd=REPOSITORY_ROOT,
            env=rejection_environment,
            capture_output=True,
            text=True,
            check=False,
            timeout=15,
        )
        assert (
            rejected.returncode == 2
            and "Both genuine slots are spent" in rejected.stderr
        )
        assert not refused_evidence.exists()
        refusals.append(
            {
                "journey": journey,
                "exit": rejected.returncode,
                "stderr": rejected.stderr,
                "credential_present": "OPENROUTER_API_KEY" in rejection_environment,
            }
        )
    write_object(evidence / "spent-slot-refusals.json", {"refusals": refusals})
    helper_checks(evidence)
    sensitivity_checks(evidence)
    pin_reader_checks()
    scripts = str(SKILL_ROOT / "scripts")
    commands = [
        ["ruff", "check", scripts],
        ["ruff", "format", "--check", scripts],
        ["basedpyright", "--pythonpath", sys.executable, scripts],
    ]
    records: list[JsonObject] = []
    for position, command in enumerate(commands):
        with (evidence / f"gate-{position + 1}.log").open("wb") as output:
            completed = subprocess.run(
                command,
                check=False,
                cwd=REPOSITORY_ROOT,
                stdout=output,
                stderr=subprocess.STDOUT,
            )
        records.append({"command": command, "exit_code": completed.returncode})
    write_object(
        evidence / "python-gates.json",
        {
            "commands": records,
            "versions": {
                name: importlib.metadata.version(name)
                for name in ["pydantic", "pydantic_core", "ruff", "basedpyright"]
            },
        },
    )
    assert all(record["exit_code"] == 0 for record in records), (
        f"Python gates failed; inspect {evidence}"
    )
    print("Standalone helpers and three Python gates passed")


if __name__ == "__main__":
    main()
