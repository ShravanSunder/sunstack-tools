"""Own target-pinned CLI processes and event-driven loopback replies for rulebook journeys."""

import os
import signal
import subprocess
import sys
import threading
import typing as t
from dataclasses import dataclass, field
from pathlib import Path

from rulebook_fixture_support import JsonObject, read_object, require, write_json

# Reuse the repository's existing target checks and HTTP capture. Neither helper
# chooses rulebook expectations, config semantics, or a product target for us.
sys.path.insert(
    0,
    str(
        Path(__file__).resolve().parents[3]
        / ".agents/skills/verify-sunstack-rule/scripts"
    ),
)
from runner_process import RunnerTarget, binary_digest, fixture_snapshot
from scenario_transport import ControlledServer, PendingRequest, response_body

type Reply = t.Callable[[JsonObject], JsonObject]


@dataclass(kw_only=True)
class RulebookProcess:
    target: RunnerTarget
    evidence: Path
    server: ControlledServer = field(init=False)
    responder: threading.Thread = field(init=False)
    errors: list[str] = field(default_factory=list)
    reply: Reply = field(default=lambda body: response_body(body, 0.91, "outcome"))
    stop_request: PendingRequest = field(init=False)
    cancel_on_request: bool = False
    child_ready: threading.Event = field(default_factory=threading.Event)
    child: subprocess.Popen[bytes] | None = None

    def __enter__(self) -> t.Self:
        self.target.confirm()
        self.evidence.mkdir(parents=True, exist_ok=False)
        self.server = ControlledServer(self.evidence / "http")
        self.stop_request = PendingRequest(body={}, raw=b"")

        def respond() -> None:
            while True:
                pending = self.server.pending.get()
                if pending is self.stop_request:
                    return
                try:
                    if self.cancel_on_request:
                        require(
                            self.child_ready.wait(timeout=5),
                            "Owned child did not start",
                        )
                        require(self.child is not None, "No child to cancel")
                        if self.child is not None:
                            os.killpg(self.child.pid, signal.SIGINT)
                        continue
                    pending.respond(200, self.reply(pending.body))
                except (ValueError, KeyError, TypeError, AssertionError) as error:
                    self.errors.append(str(error))
                    pending.respond(503, {"error": "rulebook fixture response failed"})

        self.responder = threading.Thread(target=respond, name="rulebook-responses")
        self.responder.start()
        return self

    def __exit__(self, *exception: object) -> None:
        self.server.pending.put(self.stop_request)
        self.responder.join(timeout=5)
        self.server.__exit__(None, None, None)
        require(not self.responder.is_alive(), "Owned responder did not finish")
        write_json(
            self.evidence / "harness.json",
            {"errors": self.errors, "requests": self.server.count},
        )
        self.target.confirm()
        require(not self.errors, f"Loopback harness failed: {self.errors}")

    def run(
        self,
        root: Path,
        name: str,
        arguments: list[str],
        *,
        expected_exit: int,
        credential: bool = False,
        stdin: bytes | None = None,
        cancel: bool = False,
    ) -> tuple[bytes, bytes, list[PendingRequest]]:
        self.target.confirm()
        directory = self.evidence / name
        directory.mkdir(exist_ok=False)
        home = directory / "home"
        home.mkdir()
        environment = {
            "PATH": "/usr/bin:/bin",
            "HOME": str(home),
            "LANG": "C.UTF-8",
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_CONFIG_GLOBAL": os.devnull,
            "SUNSTACK_CREDENTIAL_STORE": "off",
        }
        if credential:
            environment["OPENROUTER_API_KEY"] = "SYNTHETIC_RULEBOOK_LOOPBACK_ONLY"
        before = self.server.count
        input_files = fixture_snapshot(root)
        self.cancel_on_request = cancel
        self.child_ready.clear()
        command = [str(self.target.binary), *arguments]
        with (
            (directory / "stdout.log").open("wb") as stdout,
            (directory / "stderr.log").open("wb") as stderr,
        ):
            process = subprocess.Popen(
                command,
                cwd=root,
                env=environment,
                stdin=subprocess.PIPE,
                stdout=stdout,
                stderr=stderr,
                start_new_session=True,
            )
            self.child = process
            self.child_ready.set()
            try:
                if stdin is not None:
                    if process.stdin is None:
                        raise ValueError("Missing owned stdin")
                    process.stdin.write(stdin)
                    process.stdin.close()
                # With no input, stdin stays open: a read would block. Process
                # completion is the signal, with a bound, never a polling loop.
                exit_code = process.wait(timeout=30)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=5)
                write_json(
                    directory / "timeout.json", {"pid": process.pid, "seconds": 30}
                )
                raise
            finally:
                if process.stdin is not None and not process.stdin.closed:
                    process.stdin.close()
        captures = self.server.received[before:]
        write_json(
            directory / "process.json",
            {
                "command": command,
                "cwd": str(root),
                "pid": process.pid,
                "commit": self.target.commit,
                "binary_sha256": self.target.digest,
                "exit_code": exit_code,
                "http_requests": len(captures),
                "credential": "synthetic" if credential else "absent",
                "stdin_held_open": stdin is None,
                "cancel_on_request": cancel,
            },
        )
        self.target.confirm()
        if "--init" not in arguments:
            require(
                fixture_snapshot(root) == input_files, "Lint modified its input fixture"
            )
        require(
            exit_code == expected_exit,
            f"{name}: exit {exit_code}, expected {expected_exit}; {directory}",
        )
        return (
            (directory / "stdout.log").read_bytes(),
            (directory / "stderr.log").read_bytes(),
            captures,
        )

    def report(
        self,
        root: Path,
        name: str,
        arguments: list[str],
        *,
        expected_exit: int,
        credential: bool = False,
    ) -> tuple[JsonObject, list[PendingRequest]]:
        self.run(
            root, name, arguments, expected_exit=expected_exit, credential=credential
        )
        report = read_object(self.evidence / name / "stdout.log")
        process = read_object(self.evidence / name / "process.json")
        count = process["http_requests"]
        return report, self.server.received[-count:] if isinstance(
            count, int
        ) and count else []


def declared_target(binary: Path, source: Path, commit: str) -> RunnerTarget:
    target = RunnerTarget(
        binary=binary.resolve(),
        source=source.resolve(),
        commit=commit,
        digest=binary_digest(binary),
    )
    target.confirm()
    return target
