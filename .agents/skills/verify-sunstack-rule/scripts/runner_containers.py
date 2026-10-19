"""Own pinned container launch settings and the declared loopback route."""

import os
import re
import subprocess
import uuid
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlsplit

from runner_contracts import FixtureCase, write_json

IMAGES = {
    "amd64": {
        "alpine": "alpine@sha256:d56c381f961d307a21b3ca004cf1e3910f106644aefb1f43e654c8a56c4fd395",
        "debian": "debian@sha256:d405c1e1d9eecbf30022d23766a0eea235e7f499aa33e357dfd29271a839f82a",
    },
    "arm64": {
        "alpine": "alpine@sha256:260479a1cfaf304c4c20da7f8405d3ce313513dcd534bb743257bdd2fe0f3e2d",
        "debian": "debian@sha256:698c1aab9c6357a151053fae10191cc95f605a3eeba33999e6f7ed7a64c91f9e",
    },
}


@dataclass(frozen=True, kw_only=True)
class ContainerRuntime:
    image: str
    control_host: str

    def __post_init__(self) -> None:
        if self.image not in {images["alpine"] for images in IMAGES.values()}:
            raise ValueError("S1 needs an adopted pinned Alpine image digest")
        if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9.-]*", self.control_host):
            raise ValueError("Declare only the owned control route's hostname")

    @property
    def architecture(self) -> str:
        return next(
            architecture
            for architecture, images in IMAGES.items()
            if images["alpine"] == self.image
        )

    def confirm_image(self) -> None:
        result = subprocess.run(
            [
                "docker",
                "image",
                "inspect",
                "--platform",
                f"linux/{self.architecture}",
                self.image,
                "--format",
                "{{.Architecture}}",
            ],
            capture_output=True,
            check=True,
            timeout=15,
        )
        if result.stdout.decode().strip() != self.architecture:
            raise ValueError("Pinned S1 image has the wrong architecture")

    def control_endpoint(self, endpoint: str) -> str:
        parsed = urlsplit(endpoint)
        if (
            parsed.scheme != "http"
            or parsed.hostname != "127.0.0.1"
            or parsed.port is None
        ):
            raise ValueError("Expected the owned loopback server endpoint")
        return f"http://{self.control_host}:{parsed.port}{parsed.path}"

    def confirm_control(self, endpoint: str, evidence: Path) -> None:
        name = f"sunstack-control-{uuid.uuid4().hex}"
        command = [
            "docker",
            "run",
            "--rm",
            "--name",
            name,
            "--pull",
            "never",
            "--read-only",
            "--platform",
            f"linux/{self.architecture}",
            self.image,
            "/bin/busybox",
            "wget",
            "-S",
            "-T",
            "5",
            "-O",
            "/dev/null",
            endpoint,
        ]
        try:
            result = subprocess.run(
                command, check=False, capture_output=True, timeout=20
            )
        except subprocess.TimeoutExpired:
            cleanup = subprocess.run(
                ["docker", "rm", "--force", name],
                check=False,
                capture_output=True,
                timeout=15,
            )
            (evidence / "control-cleanup.log").write_bytes(
                cleanup.stdout + cleanup.stderr
            )
            raise TimeoutError("Owned container-to-control probe timed out") from None
        (evidence / "control-network.stdout.log").write_bytes(result.stdout)
        (evidence / "control-network.stderr.log").write_bytes(result.stderr)
        write_json(
            evidence / "control-network.json",
            {
                "command": command,
                "exit_code": result.returncode,
                "http_501_reached": b"501" in result.stderr,
            },
        )
        if b"501" not in result.stderr:
            raise ValueError("Container-to-owned-control HTTP route not established")

    def command(
        self, binary: Path, fixture: Path, case: FixtureCase, name: str
    ) -> list[str]:
        if any("," in str(path) for path in (binary, fixture)):
            raise ValueError("Container bind paths cannot contain commas")
        readonly = case.mode != "init"
        command = [
            "docker",
            "run",
            "--rm",
            "--name",
            name,
            "--pull",
            "never",
            "--read-only",
            "--platform",
            f"linux/{self.architecture}",
            "--cap-drop",
            "ALL",
            "--security-opt",
            "no-new-privileges",
            "--tmpfs",
            "/home/runner:rw,nosuid,nodev,size=16m",
            "--workdir",
            str(Path("/fixture") / case.cwd),
            "--mount",
            f"type=bind,src={binary},dst=/sunstack,readonly",
            "--mount",
            f"type=bind,src={fixture},dst=/fixture" + (",readonly" if readonly else ""),
        ]
        if case.requests == 0:
            command.extend(["--network", "none"])
        command.extend(
            [
                self.image,
                "/usr/bin/env",
                "-i",
                "PATH=/usr/bin:/bin",
                "HOME=/home/runner",
            ]
        )
        if case.credential:
            command.append("OPENROUTER_API_KEY=SYNTHETIC_RUNNER_CONTROL_NOT_A_SECRET")
        return [*command, "/sunstack", *case.arguments]


def docker_client_environment() -> dict[str, str]:
    environment = {
        "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
        "HOME": os.environ.get("HOME", "/tmp"),
    }
    for key in ("DOCKER_HOST", "DOCKER_CONTEXT", "DOCKER_CONFIG"):
        if key in os.environ:
            environment[key] = os.environ[key]
    return environment
