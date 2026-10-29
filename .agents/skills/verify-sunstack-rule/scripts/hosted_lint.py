# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic==2.14.1", "pydantic_core==2.50.1"]
# ///
"""S3: separately commissioned genuine K001/K003 rulebook lint in arm64 Alpine.

--prepare --evidence <new-dir> builds the adopted S0 fixture without a key.
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
    JsonObject,
    as_array,
    as_object,
    read_object,
    write_json,
)
from runner_linkage import inspect_elf
from runner_process import binary_digest

sys.path.insert(
    0, str(Path(__file__).resolve().parents[4] / "scripts/verify/sunstack-rule")
)
from rulebook_fixture_support import (
    CONFIG,
    configure,
    prepare_fixture,
    select_worked_skill,
)


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
        prepare_fixture(fixture)
        select_worked_skill(fixture)

        def hosted_settings(config: JsonObject) -> None:
            config["decision_model"] = {"model": "jev-latest"}
            config["limits"] = {
                "max_model_requests": 16,
                "max_concurrent_requests": 1,
                "attempt_timeout_seconds": 60,
            }

        configure(fixture, hosted_settings)
        if arguments.prepare:
            write_json(
                evidence / "s3-prepared.json",
                {"product_observed": False, "genuine_requests": 0},
            )
            print(
                "Prepared S3 rulebook fixture; no credential read or product observed"
            )
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
            f'test -z "$(find /etc/ssl -type f -print)" || exit 92; printf "CA_BUNDLE_EMPTY\\n" >&2; exec /usr/bin/env -i PATH=/usr/bin:/bin HOME=/home/runner OPENROUTER_API_KEY="$OPENROUTER_API_KEY" /sunstack lint --config {CONFIG} --all --format json',
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
        except subprocess.TimeoutExpired as error:
            partial_stdout = error.stdout or b""
            partial_stderr = error.stderr or b""
            leaked = (
                secret.encode() in partial_stdout or secret.encode() in partial_stderr
            )
            del secret
            timeout_receipt: JsonObject = {
                "remote_outcome": "unknown",
                "container_cleanup_exit": None,
                "container_cleanup_status": "unknown",
                "retry_performed": False,
                "output_partial": True,
                "raw_output_retained": not leaked,
            }
            write_json(
                evidence / "s3-timeout.json",
                timeout_receipt,
            )
            if leaked:
                write_json(
                    evidence / "credential-leak.json",
                    {"observed": True, "raw_output_retained": False},
                )
            else:
                (evidence / "stdout.partial.log").write_bytes(partial_stdout)
                (evidence / "stderr.partial.log").write_bytes(partial_stderr)
            # Safe evidence already exists if owned cleanup cannot complete.
            try:
                cleanup = subprocess.run(
                    ["docker", "rm", "--force", name],
                    capture_output=True,
                    timeout=15,
                    check=False,
                )
                timeout_receipt["container_cleanup_exit"] = cleanup.returncode
                timeout_receipt["container_cleanup_status"] = (
                    "completed" if cleanup.returncode == 0 else "failed"
                )
            except subprocess.TimeoutExpired:
                timeout_receipt["container_cleanup_status"] = "timed_out"
            except OSError:
                timeout_receipt["container_cleanup_status"] = "launch_failed"
            write_json(evidence / "s3-timeout.json", timeout_receipt)
            if leaked:
                raise ValueError(
                    "Credential appeared in partial output; raw output withheld; "
                    "remote billing/outcome unknown; do not retry"
                ) from None
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
                "Hosted rulebook lint did not return expected K002 error with empty CA bundle"
            )
        report = read_object(evidence / "stdout.json")
        usage = as_object(report["usage"])
        if report["version"] != arguments.version or not usage.get("input_tokens"):
            raise ValueError("Wrong build version or missing real inference usage")
        if as_array(report["incomplete"]) or as_array(report["skipped_rules"]):
            raise ValueError("Hosted rulebook report is incomplete or skipped rules")
        states = [
            as_object(value) for value in as_object(report["evidence_seen"]).values()
        ]
        standards = {str(state["standard"]) for state in states}
        expected_standards = {
            "Each workflow step states a checkable completion covering its obligations.",
            "A skill description names when to use the skill.",
        }
        if not expected_standards.issubset(standards):
            raise ValueError("Real admitted requests did not cover K001 and K003")
        if binary_digest(binary) != arguments.sha256:
            raise ValueError("Static binary changed during hosted run")
        write_json(
            evidence / "s3-pass.json",
            {
                "commit": arguments.commit,
                "binary_sha256": arguments.sha256,
                "genuine_fixture": "rulebook K001 and K003 with mechanical K002 error",
                "usage": usage,
                "image": image,
                "gaps": [
                    "one fixture/model only",
                    "private CAs unverified",
                    "semantic answer quality is not judged",
                ],
            },
        )
        print("S3 rulebook hosted lint observed; evidence retained; no retry performed")
    except (ValueError, TypeError, OSError, subprocess.SubprocessError) as error:
        parser.exit(2, f"S3 setup/observation failed: {error}\n")


if __name__ == "__main__":
    main()
