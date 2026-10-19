# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic==2.14.1", "pydantic_core==2.50.1"]
# ///
"""S3: one separately commissioned genuine Jev lint in static arm64 Alpine.

--prepare --evidence <new-dir> builds the known-false fixture without a key.
Observation requires --binary, --sha256, --commit, --version and an authorized
--credential-file (owner-only mode 600). No automatic rerun. The credential is
read once into only the docker command's environment and is never an argument
to Docker, a stored command, a fixture, or output. /etc/ssl is empty tmpfs.
"""

import argparse
import os
import re
import stat
import subprocess
import sys
import uuid
from pathlib import Path

from container_matrix import IMAGES
from runner_contracts import (
    FixtureCase,
    as_array,
    as_object,
    compare_report,
    read_object,
    write_json,
)
from runner_fixture_setup import (
    base_config,
    expected_finding,
    judged_rule,
    write_fixture,
)
from runner_linkage import inspect_elf
from runner_process import binary_digest


def read_credential(path: Path) -> str:
    try:
        metadata = path.lstat()
        if (
            not stat.S_ISREG(metadata.st_mode)
            or stat.S_IMODE(metadata.st_mode) != 0o600
            or metadata.st_uid != os.getuid()
        ):
            raise ValueError("Hosted credential must be an owner-only regular file")
        secret = path.read_text().strip()
        if not secret or "\n" in secret or "\r" in secret:
            raise ValueError("Hosted credential is empty or multiline")
        return secret
    except OSError:
        raise ValueError("Cannot read the authorized hosted credential") from None


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--prepare", action="store_true")
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--sha256")
    parser.add_argument("--commit")
    parser.add_argument("--version")
    parser.add_argument("--credential-file", type=Path)
    arguments = parser.parse_args()
    if sys.flags.optimize:
        parser.error("Unset PYTHONOPTIMIZE and omit -O/-OO")
    if not arguments.prepare and not all(
        (
            arguments.binary,
            arguments.sha256,
            arguments.commit,
            arguments.version,
            arguments.credential_file,
        )
    ):
        parser.error(
            "Hosted observation needs all target fields and --credential-file; --prepare is offline"
        )
    try:
        binary = Path()
        if not arguments.prepare:
            binary = arguments.binary.resolve()
            if binary_digest(binary) != arguments.sha256:
                raise ValueError("Static binary differs from declared SHA256")
            if not re.fullmatch(r"[0-9a-f]{40}", arguments.commit):
                raise ValueError("Declare a full source commit")
            inspect_elf(binary.read_bytes(), "arm64")
        evidence = arguments.evidence.resolve()
        evidence.mkdir(parents=True, exist_ok=False)
        fixture = evidence / "fixture"
        config = base_config()
        config["backend"] = {"kind": "jev_openrouter", "model": "jev-latest"}
        config["limits"] = {
            "max_model_requests": 1,
            "max_concurrent_requests": 1,
            "attempt_timeout_seconds": 60,
        }
        write_fixture(fixture, config=config, rules=[judged_rule()])
        (fixture / "docs/one.md").write_text(
            "U001: The integer 2 plus the integer 2 equals the integer 5.\n"
        )
        if arguments.prepare:
            write_json(
                evidence / "s3-prepared.json",
                {"product_observed": False, "genuine_requests": 0},
            )
            print("Prepared S3 fixture; no credential read or product observed")
            return
        image = IMAGES["arm64"]["alpine"]
        subprocess.run(
            ["docker", "image", "inspect", "--platform", "linux/arm64", image],
            capture_output=True,
            timeout=15,
            check=True,
        )
        name = f"sunstack-hosted-{uuid.uuid4().hex}"
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
            "linux/arm64",
            "--cap-drop",
            "ALL",
            "--security-opt",
            "no-new-privileges",
            "--workdir",
            "/fixture",
            "--tmpfs",
            "/etc/ssl:rw,nosuid,nodev,noexec,size=1m",
            "--tmpfs",
            "/home/runner:rw,nosuid,nodev,size=1m",
            "--mount",
            f"type=bind,src={binary},dst=/sunstack,readonly",
            "--mount",
            f"type=bind,src={fixture},dst=/fixture,readonly",
            "--env",
            "OPENROUTER_API_KEY",
            image,
            "/bin/sh",
            "-c",
            'test -z "$(find /etc/ssl -type f -print)" || exit 92; printf "CA_BUNDLE_EMPTY\\n" >&2; exec /usr/bin/env -i PATH=/usr/bin:/bin HOME=/home/runner OPENROUTER_API_KEY="$OPENROUTER_API_KEY" /sunstack lint --all --format json',
        ]
        secret = read_credential(arguments.credential_file)
        environment = {
            "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
            "HOME": os.environ.get("HOME", "/tmp"),
            "OPENROUTER_API_KEY": secret,
        }
        # Keep Docker client connection selection, never proxy or provider settings.
        for name_key in ("DOCKER_HOST", "DOCKER_CONTEXT", "DOCKER_CONFIG"):
            if name_key in os.environ:
                environment[name_key] = os.environ[name_key]
        try:
            result = subprocess.run(
                command, env=environment, capture_output=True, timeout=240, check=False
            )
        except subprocess.TimeoutExpired:
            cleanup = subprocess.run(
                ["docker", "rm", "--force", name],
                capture_output=True,
                timeout=15,
                check=False,
            )
            write_json(
                evidence / "s3-timeout.json",
                {
                    "remote_outcome": "unknown",
                    "container_cleanup_exit": cleanup.returncode,
                    "retry_performed": False,
                },
            )
            raise ValueError(
                "Hosted command timed out; remote billing/outcome unknown; do not retry"
            ) from None
        finally:
            environment.pop("OPENROUTER_API_KEY", None)
        leaked = secret.encode() in result.stdout or secret.encode() in result.stderr
        del secret
        if leaked:
            write_json(
                evidence / "credential-leak.json",
                {"observed": True, "raw_output_retained": False},
            )
            raise ValueError("Credential appeared in output; raw output withheld")
        (evidence / "stdout.json").write_bytes(result.stdout)
        (evidence / "stderr.log").write_bytes(result.stderr)
        write_json(
            evidence / "process.json",
            {
                "command": command,
                "commit": arguments.commit,
                "binary_sha256": arguments.sha256,
                "version": arguments.version,
                "exit_code": result.returncode,
                "retry_performed": False,
            },
        )
        if result.returncode != 1 or b"CA_BUNDLE_EMPTY" not in result.stderr:
            raise ValueError(
                "Hosted known-false lint did not return expected finding with empty CA bundle"
            )
        report = read_object(evidence / "stdout.json")
        case = FixtureCase(
            name="hosted-known-false",
            findings=[expected_finding(kind="judged")],
            requests=1,
            credential=True,
            cost=False,
            wrong_build="TLS depends on OS roots or known-false judgment is lost",
        )
        # Provider cost is optional; compare its actual presence separately below.
        usage = as_object(report["usage"])
        case = case.model_copy(update={"cost": usage.get("cost_usd") is not None})
        compare_report(report, case, Path("/fixture"))
        if report["version"] != arguments.version or not usage.get("input_tokens"):
            raise ValueError("Wrong build version or missing real inference usage")
        for raw in as_array(report["findings"]):
            finding = as_object(raw)
            judgment = as_object(finding["judgment"])
            if (
                judgment["result"] != "fail"
                and as_object(judgment["result"])["kind"] != "fail"
            ):
                raise ValueError("Known-false hosted verdict did not fail")
        if binary_digest(binary) != arguments.sha256:
            raise ValueError("Static binary changed during hosted run")
        write_json(
            evidence / "s3-pass.json",
            {
                "commit": arguments.commit,
                "binary_sha256": arguments.sha256,
                "genuine_fixture": "known-false arithmetic",
                "usage": usage,
                "image": image,
                "gaps": ["one fixture/model only", "private CAs unverified"],
            },
        )
        print(
            "S3 known-false hosted lint observed; evidence retained; no retry performed"
        )
    except (ValueError, TypeError, OSError, subprocess.SubprocessError) as error:
        parser.exit(2, f"S3 setup/observation failed: {error}\n")


if __name__ == "__main__":
    main()
