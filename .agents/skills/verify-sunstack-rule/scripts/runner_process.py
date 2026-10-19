"""Own bounded runner children, pin their binary/source, and retain safe evidence."""

import hashlib
import os
import re
import signal
import subprocess
import time
import uuid
from dataclasses import dataclass
from pathlib import Path
from queue import Empty

from runner_containers import ContainerRuntime, docker_client_environment
from runner_contracts import FixtureCase, write_json
from runner_linkage import inspect_elf
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
    container: ContainerRuntime | None = None

    def comparison_root(self, fixture: Path) -> Path:
        return Path("/fixture") if self.container else fixture

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
        if self.container:
            inspect_elf(self.binary.read_bytes(), self.container.architecture)
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
    container_name = None
    if target.container:
        container_name = f"sunstack-s1-{uuid.uuid4().hex}"
        command = target.container.command(target.binary, root, case, container_name)
        environment = docker_client_environment()
    unreadable = root / case.unreadable if case.unreadable else None
    before = fixture_snapshot(root)
    if unreadable:
        unreadable.chmod(0)
    initial_requests = server.count
    started = time.monotonic()
    pending_requests = 0
    signals: list[dict[str, object]] = []
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
                        if container_name:
                            signal_command = [
                                "docker",
                                "kill",
                                "--signal=SIGINT",
                                container_name,
                            ]
                            delivered = subprocess.run(
                                signal_command,
                                check=False,
                                capture_output=True,
                                timeout=10,
                            )
                            signals.append(
                                {
                                    "command": signal_command,
                                    "exit_code": delivered.returncode,
                                }
                            )
                            if delivered.returncode:
                                raise ValueError("Owned container signal failed")
                        else:
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
                    if container_name:
                        cleanup = subprocess.run(
                            ["docker", "rm", "--force", container_name],
                            check=False,
                            capture_output=True,
                            timeout=15,
                        )
                        (directory / "cleanup.log").write_bytes(
                            cleanup.stdout + cleanup.stderr
                        )
                    else:
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
                "container": container_name,
                "fixture_readonly": bool(target.container and case.mode != "init"),
                "signals": signals,
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
