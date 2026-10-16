"""Own one real CLI child and retain its caller-visible protocol evidence."""

import hashlib
import json
import os
import re
import subprocess
import time
import types
import typing as t
from contextlib import ExitStack
from dataclasses import dataclass, field
from pathlib import Path

from pydantic import BaseModel, ConfigDict, JsonValue, TypeAdapter

type JsonObject = dict[str, JsonValue]
type OperationName = t.Literal["prepare_evidence", "prepare_rules", "evaluate_rules"]
type CredentialMode = t.Literal["none", "synthetic", "hosted"]
JSON_OBJECT = TypeAdapter(JsonObject)
SKILL_ROOT = Path(__file__).resolve().parent.parent
REPOSITORY_ROOT = SKILL_ROOT.parents[2]
INDEX_VARIABLES = (
    "UV_INDEX",
    "UV_EXTRA_INDEX_URL",
    "UV_INDEX_URL",
    "UV_FIND_LINKS",
    "PIP_INDEX_URL",
    "PIP_EXTRA_INDEX_URL",
    "UV_DEFAULT_INDEX",
    "UV_CONFIG_FILE",
    "UV_ENV_FILE",
)


def as_object(value: JsonValue) -> JsonObject:
    assert isinstance(value, dict), "Expected a JSON object"
    return value


def as_array(value: JsonValue) -> list[JsonValue]:
    assert isinstance(value, list), "Expected a JSON array"
    return value


def as_text(value: JsonValue) -> str:
    assert isinstance(value, str), "Expected JSON text"
    return value


def object_at(value: JsonValue, *keys: str) -> JsonObject:
    for key in keys:
        value = as_object(value)[key]
    return as_object(value)


def array_at(value: JsonValue, *keys: str) -> list[JsonValue]:
    for key in keys:
        value = as_object(value)[key]
    return as_array(value)


def read_object(path: Path) -> JsonObject:
    return JSON_OBJECT.validate_json(path.read_bytes())


def write_object(path: Path, value: JsonObject) -> None:
    path.write_text(
        json.dumps(value, indent=None if path.suffix == ".jsonl" else 2) + "\n"
    )


class NativeOperation(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True)
    operation: OperationName
    outcome: JsonObject


class OperationFrame(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True)
    kind: t.Literal["operation_result"]
    protocol_version: t.Literal[1]
    result: NativeOperation


class ProcessRecord(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True)
    command: list[str]
    pid: int
    cwd: str
    source_head: str
    product_commit: str
    binary_sha256: str
    operation: OperationName
    exit_code: int
    elapsed_ms: float
    unsigned_warning: bool


@dataclass(frozen=True, kw_only=True)
class OperationRequest:
    name: str
    operation: OperationName
    input_value: JsonObject
    cwd: Path
    expected_exit: int = 0
    credential: CredentialMode = "synthetic"


@dataclass(kw_only=True)
class DriverContext:
    binary: Path
    evidence: Path
    journey: str
    product_commit: str
    source_head: str
    step_number: int = 0
    records: list[ProcessRecord] = field(default_factory=list)

    @property
    def directory(self) -> Path:
        return self.evidence / self.journey

    def begin(self, request: OperationRequest) -> "RunningProcess":
        self.step_number += 1
        directory = self.directory / f"{self.step_number:02d}-{request.name}"
        directory.mkdir(parents=True, exist_ok=False)
        write_object(
            directory / "stdin.json",
            {
                "protocol_version": 1,
                "operation": request.operation,
                "input": request.input_value,
            },
        )
        environment = os.environ.copy()
        environment.pop("OPENROUTER_API_KEY", None)
        if request.credential == "synthetic":
            environment["OPENROUTER_API_KEY"] = "SYNTHETIC_SCENARIO_CREDENTIAL"
            endpoint = as_text(object_at(request.input_value, "backend")["endpoint"])
            assert endpoint.startswith("http://127.0.0.1:"), (
                "Local scenarios require owned loopback"
            )
        elif request.credential == "hosted":
            key = os.environ.get("OPENROUTER_API_KEY")
            assert key, "The authorized hosted command needs its in-memory credential"
            assert object_at(request.input_value, "request_budget")["max_requests"] == 1
            assert object_at(request.input_value, "attempts")["retry_count"] == 0
            assert (
                object_at(request.input_value, "backend")["endpoint"]
                == "https://openrouter.ai/api/v1/systemone"
            )
            environment["OPENROUTER_API_KEY"] = key
        stack = ExitStack()
        try:
            stdin = stack.enter_context((directory / "stdin.json").open("rb"))
            stdout = stack.enter_context((directory / "stdout.json").open("wb"))
            stderr = stack.enter_context((directory / "stderr.log").open("wb"))
            command = [
                str(REPOSITORY_ROOT / "scripts/cargo-debug-signing-runner.sh"),
                str(self.binary),
            ]
            started = time.monotonic()
            process = subprocess.Popen(
                command,
                stdin=stdin,
                stdout=stdout,
                stderr=stderr,
                cwd=request.cwd,
                env=environment,
            )
        except OSError:
            stack.close()
            raise
        return RunningProcess(
            context=self,
            directory=directory,
            operation=request.operation,
            process=process,
            stack=stack,
            started=started,
            command=command,
            cwd=request.cwd,
            deadline=90 if request.credential == "hosted" else 15,
        )


@dataclass(kw_only=True)
class RunningProcess:
    context: DriverContext
    directory: Path
    operation: OperationName
    process: subprocess.Popen[bytes]
    stack: ExitStack
    started: float
    command: list[str]
    cwd: Path
    deadline: int
    collected: bool = False

    def __enter__(self) -> t.Self:
        return self

    def __exit__(
        self,
        exception_type: type[BaseException] | None,
        exception: BaseException | None,
        traceback: types.TracebackType | None,
    ) -> None:
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)
        self.stack.close()
        if not self.collected:
            self.record()

    def record(self) -> ProcessRecord:
        stderr = (self.directory / "stderr.log").read_text()
        return_code = self.process.poll()
        assert return_code is not None
        record = ProcessRecord(
            command=self.command,
            pid=self.process.pid,
            cwd=str(self.cwd),
            source_head=self.context.source_head,
            product_commit=self.context.product_commit,
            binary_sha256=hashlib.sha256(self.context.binary.read_bytes()).hexdigest(),
            operation=self.operation,
            exit_code=return_code,
            elapsed_ms=(time.monotonic() - self.started) * 1000,
            unsigned_warning="runs unsigned" in stderr,
        )
        (self.directory / "result.json").write_text(
            record.model_dump_json(indent=2) + "\n"
        )
        self.context.records.append(record)
        self.collected = True
        return record

    def finish(self, expected_exit: int) -> JsonObject:
        self.process.wait(timeout=self.deadline)
        self.stack.close()
        record = self.record()
        assert record.exit_code == expected_exit, (
            f"Unexpected exit; inspect {self.directory}"
        )
        stderr = (self.directory / "stderr.log").read_text()
        assert "sunstack-rule: ready\n" in stderr, (
            "Signal registration was not observed"
        )
        assert len(stderr.encode()) <= 256, "Unbounded operational stderr"
        frame = OperationFrame.model_validate_json(
            (self.directory / "stdout.json").read_bytes()
        )
        assert frame.result.operation == self.operation
        return frame.result.outcome


def run_operation(context: DriverContext, request: OperationRequest) -> JsonObject:
    with context.begin(request) as process:
        return process.finish(request.expected_exit)


def complete_report(outcome: JsonObject) -> JsonObject:
    assert outcome["kind"] == "complete_report"
    return object_at(outcome, "report")


def checked_source() -> tuple[str, str]:
    text = (SKILL_ROOT / "SKILL.md").read_text()
    match = re.search(r"milestone 1 product target is `([0-9a-f]{40})`", text)
    assert match, "Main skill must declare its product pin"
    product_commit = match.group(1)
    source_head = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=REPOSITORY_ROOT, text=True
    ).strip()
    subprocess.run(
        [
            "git",
            "diff",
            "--quiet",
            product_commit,
            "--",
            "crates",
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            ".cargo",
            "scripts",
        ],
        cwd=REPOSITORY_ROOT,
        check=True,
    )
    return product_commit, source_head
