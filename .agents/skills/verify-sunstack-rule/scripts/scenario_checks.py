# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic==2.14.1", "pydantic_core==2.50.1", "ruff==0.17.1", "basedpyright==1.40.2"]
# ///
"""Standalone helper checks and exact pinned Python quality gates."""

import argparse
import importlib.metadata
import json
import os
import subprocess
import sys
import threading
import urllib.request
from pathlib import Path
from queue import Queue

from pydantic import ValidationError
from scenario_runtime import (
    INDEX_VARIABLES,
    REPOSITORY_ROOT,
    SKILL_ROOT,
    JsonObject,
    OperationFrame,
    write_object,
)
from scenario_transport import ControlledServer, response_body


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
            "duplicate_response_refused": True,
            "owned_threads_joined": True,
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
    helper_checks(evidence)
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
