"""Own bounded runner children, pin their binary/source, and retain safe evidence."""

import hashlib
import os
import re
import signal
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from queue import Empty

from runner_contracts import FixtureCase, write_json
from scenario_transport import ControlledServer, response_body


def binary_digest(binary: Path) -> str:
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise ValueError("Declared binary is missing or not executable")
    return hashlib.sha256(binary.read_bytes()).hexdigest()


@dataclass(frozen=True, kw_only=True)
class RunnerTarget:
    binary: Path
    source: Path
    commit: str
    digest: str

    def confirm(self) -> None:
        if not re.fullmatch(r"[0-9a-f]{40}", self.commit):
            raise ValueError("--commit must be a full Git commit")
        head = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=self.source,
            capture_output=True,
            text=True,
            check=True,
            timeout=10,
        ).stdout.strip()
        status = subprocess.run(
            [
                "git",
                "status",
                "--porcelain",
                "--",
                "crates",
                "Cargo.toml",
                "Cargo.lock",
                "rust-toolchain.toml",
                ".cargo",
            ],
            cwd=self.source,
            capture_output=True,
            text=True,
            check=True,
            timeout=10,
        ).stdout
        if head != self.commit or status:
            raise ValueError("Declared product source changed or is dirty")
        if binary_digest(self.binary) != self.digest:
            raise ValueError("Declared binary bytes changed")


def fixture_snapshot(root: Path) -> dict[str, str]:
    return {
        path.relative_to(root).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in root.rglob("*")
        if path.is_file() and ".git" not in path.relative_to(root).parts
    }


def run_case(
    target: RunnerTarget,
    root: Path,
    directory: Path,
    case: FixtureCase,
    server: ControlledServer,
) -> tuple[int, bytes, bytes]:
    target.confirm()
    directory.mkdir(parents=True, exist_ok=False)
    command = [str(target.binary), *case.arguments]
    environment = {
        "PATH": "/usr/bin:/bin",
        "HOME": str(directory / "home"),
        "LANG": "C",
        "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_CONFIG_GLOBAL": os.devnull,
    }
    (directory / "home").mkdir()
    if case.credential:
        environment["OPENROUTER_API_KEY"] = "SYNTHETIC_RUNNER_CONTROL_NOT_A_SECRET"
    unreadable = root / case.unreadable if case.unreadable else None
    before = fixture_snapshot(root)
    if unreadable:
        unreadable.chmod(0)
    initial_requests = server.count
    started = time.monotonic()
    pending_requests = 0
    try:
        with (
            (directory / "stdout.log").open("wb") as stdout,
            (directory / "stderr.log").open("wb") as stderr,
        ):
            process = subprocess.Popen(
                command,
                cwd=root / case.cwd,
                env=environment,
                stdout=stdout,
                stderr=stderr,
                start_new_session=True,
            )
            try:
                while process.poll() is None:
                    if time.monotonic() - started > 30:
                        raise TimeoutError(
                            f"{case.name}: runner did not settle in 30 seconds"
                        )
                    try:
                        pending = server.pending.get(timeout=0.05)
                    except Empty:
                        continue
                    pending_requests += 1
                    if case.cancel:
                        os.killpg(process.pid, signal.SIGINT)
                    else:
                        response = response_body(pending.body, case.probability, "none")
                        response["model"] = "fixture/runner-control"
                        if not case.cost:
                            usage = response["usage"]
                            if isinstance(usage, dict):
                                usage.pop("cost", None)
                        pending.respond(200 if case.requests else 503, response)
                exit_code = process.wait(timeout=5)
            finally:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=5)
        write_json(
            directory / "process.json",
            {
                "command": command,
                "cwd": str(root / case.cwd),
                "pid": process.pid,
                "commit": target.commit,
                "binary_sha256": target.digest,
                "exit_code": exit_code,
                "elapsed_ms": round((time.monotonic() - started) * 1000),
                "http_requests": server.count - initial_requests,
                "credential": "synthetic" if case.credential else "absent",
            },
        )
    finally:
        if unreadable:
            unreadable.chmod(0o644)
    target.confirm()
    if before is not None and case.mode != "init" and fixture_snapshot(root) != before:
        raise ValueError("Lint modified fixture repository")
    if (
        server.count - initial_requests != case.requests
        or pending_requests != case.requests
    ):
        raise ValueError(f"{case.name}: unexpected HTTP request count")
    if exit_code != case.exit_code:
        raise ValueError(
            f"{case.name}: exit {exit_code}, expected {case.exit_code}; inspect {directory}"
        )
    return (
        exit_code,
        (directory / "stdout.log").read_bytes(),
        (directory / "stderr.log").read_bytes(),
    )
