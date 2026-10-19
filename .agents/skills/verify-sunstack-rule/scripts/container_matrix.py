# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic==2.14.1", "pydantic_core==2.50.1"]
# ///
"""S2: exact Linux release binaries in four offline, read-only container cells.

--prepare --evidence <new-dir> writes only smoke fixtures and matrix pins.
For observation add --linux-amd64 <binary> --linux-arm64 <binary> --version
<release-version>. Optional --macos-arm64/--macos-x64 inspect shipped Mach-O
linkage. This driver never pulls an image or builds a product. Cache the exact
pins separately under the observation grant. Fresh macOS runtime is S4.
"""

import argparse
import hashlib
import subprocess
import sys
import uuid
from pathlib import Path

from runner_containers import IMAGES
from runner_contracts import (
    FixtureCase,
    compare_report,
    read_object,
    write_json,
)
from runner_fixture_setup import expected_finding, write_fixture
from runner_linkage import inspect_elf, inspect_macos
from runner_process import binary_digest


def container_command(
    binary: Path,
    fixture: Path,
    image: str,
    architecture: str,
    name: str,
    arguments: list[str],
) -> list[str]:
    if "@sha256:" not in image or len(image.rsplit(":", 1)[1]) != 64:
        raise ValueError("An immutable sha256 image digest is required")
    if any("," in str(path) for path in (binary, fixture)):
        raise ValueError("Docker bind paths cannot contain commas")
    return [
        "docker",
        "run",
        "--rm",
        "--name",
        name,
        "--pull",
        "never",
        "--read-only",
        "--network",
        "none",
        "--platform",
        f"linux/{architecture}",
        "--cap-drop",
        "ALL",
        "--security-opt",
        "no-new-privileges",
        "--tmpfs",
        "/home/runner:rw,nosuid,nodev,size=16m",
        "--workdir",
        "/fixture",
        "--mount",
        f"type=bind,src={binary},dst=/sunstack,readonly",
        "--mount",
        f"type=bind,src={fixture},dst=/fixture,readonly",
        image,
        "/usr/bin/env",
        "-i",
        "PATH=/usr/bin:/bin",
        "HOME=/home/runner",
        "/sunstack",
        *arguments,
    ]


def run_container(command: list[str], directory: Path, expected_exit: int) -> bytes:
    directory.mkdir(parents=True, exist_ok=False)
    name = command[command.index("--name") + 1]
    try:
        result = subprocess.run(command, capture_output=True, timeout=120, check=False)
    except subprocess.TimeoutExpired as error:
        cleanup = subprocess.run(
            ["docker", "rm", "--force", name],
            capture_output=True,
            timeout=15,
            check=False,
        )
        (directory / "cleanup.log").write_bytes(cleanup.stdout + cleanup.stderr)
        (directory / "stdout.log").write_bytes(error.stdout or b"")
        (directory / "stderr.log").write_bytes(error.stderr or b"")
        raise TimeoutError(
            f"Owned container {name} timed out; cleanup exit {cleanup.returncode}"
        ) from None
    (directory / "stdout.log").write_bytes(result.stdout)
    (directory / "stderr.log").write_bytes(result.stderr)
    write_json(
        directory / "process.json",
        {
            "command": command,
            "exit_code": result.returncode,
            "expected_exit": expected_exit,
        },
    )
    if result.returncode != expected_exit:
        raise ValueError(
            f"Container exit {result.returncode}, expected {expected_exit}; inspect {directory}"
        )
    return result.stdout


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--prepare", action="store_true")
    parser.add_argument("--linux-amd64", type=Path)
    parser.add_argument("--linux-arm64", type=Path)
    parser.add_argument("--macos-arm64", type=Path)
    parser.add_argument("--macos-x64", type=Path)
    parser.add_argument("--version")
    arguments = parser.parse_args()
    if sys.flags.optimize:
        parser.error("Unset PYTHONOPTIMIZE and omit -O/-OO")
    if not arguments.prepare and not all(
        (arguments.linux_amd64, arguments.linux_arm64, arguments.version)
    ):
        parser.error(
            "Observation needs both Linux binaries and --version; --prepare only builds fixtures"
        )
    try:
        binaries: dict[str, Path] = {}
        digests: dict[str, str] = {}
        if not arguments.prepare:
            for architecture, binary in (
                ("amd64", arguments.linux_amd64),
                ("arm64", arguments.linux_arm64),
            ):
                binaries[architecture] = binary.resolve()
                digests[architecture] = binary_digest(binary)
        evidence = arguments.evidence.resolve()
        evidence.mkdir(parents=True, exist_ok=False)
        fixture = evidence / "fixture"
        write_fixture(fixture)
        write_json(
            evidence / "matrix-pins.json",
            {
                "images": IMAGES,
                "provenance": "Docker image inspect --platform and immutable per-platform manifests; first-party distribution experiment pins",
            },
        )
        if arguments.prepare:
            write_json(
                evidence / "s2-prepared.json", {"product_observed": False, "cells": 4}
            )
            print("Prepared smoke fixture and 4 pinned cells; no product observed")
            return
        for architecture, binary in binaries.items():
            write_json(
                evidence / f"{architecture}-linkage.json",
                inspect_elf(binary.read_bytes(), architecture),
            )
            for distribution, image in IMAGES[architecture].items():
                metadata = subprocess.run(
                    [
                        "docker",
                        "image",
                        "inspect",
                        "--platform",
                        f"linux/{architecture}",
                        image,
                        "--format",
                        "{{json .}}",
                    ],
                    capture_output=True,
                    check=True,
                    timeout=15,
                )
                (evidence / f"{architecture}-{distribution}-image.json").write_bytes(
                    metadata.stdout
                )
                image_record = read_object(
                    evidence / f"{architecture}-{distribution}-image.json"
                )
                if image_record["Architecture"] != architecture:
                    raise ValueError("Pinned Docker image architecture mismatch")
                for label, argv, exit_code in (
                    ("version", ["--version"], 0),
                    ("check", ["lint", "--check"], 0),
                    ("lint", ["lint", "--all", "--format", "json"], 1),
                ):
                    name = f"sunstack-verify-{uuid.uuid4().hex}"
                    command = container_command(
                        binary, fixture, image, architecture, name, argv
                    )
                    directory = evidence / f"{architecture}-{distribution}-{label}"
                    output = run_container(command, directory, exit_code)
                    if (
                        label == "version"
                        and output.decode().strip() != f"sunstack {arguments.version}"
                    ):
                        raise ValueError("Release version mismatch")
                    if label == "lint":
                        case = FixtureCase(
                            name="container-smoke",
                            findings=[expected_finding()],
                            wrong_build="Container wrongly exits clean on mechanical error",
                        )
                        report = read_object(directory / "stdout.log")
                        compare_report(report, case, Path("/fixture"))
                    if (
                        hashlib.sha256(binary.read_bytes()).hexdigest()
                        != digests[architecture]
                    ):
                        raise ValueError("Release binary changed during matrix")
        macos: dict[str, object] = {}
        for architecture, binary in (
            ("arm64", arguments.macos_arm64),
            ("x64", arguments.macos_x64),
        ):
            if binary:
                result = inspect_macos(binary.resolve())
                result["sha256"] = binary_digest(binary)
                write_json(evidence / f"macos-{architecture}-linkage.json", result)
                macos[architecture] = True
        write_json(
            evidence / "s2-pass.json",
            {
                "cells": 4,
                "version": arguments.version,
                "commands": 12,
                "linux_binary_sha256": digests,
                "macos_linkage": list(macos),
                "gaps": [
                    "Fresh macOS minimal-environment runtime and Rosetta are S4",
                    "Cargo vendored/no-network feature proof must be joined from the declared source",
                ],
            },
        )
        print(
            "S2 Linux matrix passed; macOS runtime and Cargo feature proof remain separate"
        )
    except (
        ValueError,
        TypeError,
        OSError,
        TimeoutError,
        subprocess.SubprocessError,
    ) as error:
        parser.exit(2, f"S2 setup/observation failed: {error}\n")


if __name__ == "__main__":
    main()
