"""Keep safe partial evidence when a real child hangs; never use a provider key."""

import hashlib
import json
import os
import struct
import subprocess
import sys
from pathlib import Path

import hosted_lint
import pytest
from pytest_mock import MockerFixture


@pytest.mark.parametrize(
    ("leak_stream", "times_out", "cleanup_failure"),
    [
        ("none", True, "none"),
        ("stdout", True, "none"),
        ("stderr", True, "none"),
        ("empty", True, "none"),
        ("none", False, "none"),
        ("stdout", False, "none"),
        ("stderr", False, "none"),
        ("none", True, "timeout"),
        ("stdout", True, "timeout"),
        ("none", True, "launch"),
        ("stdout", True, "launch"),
    ],
)
def test_command_output_is_safe_on_timeout_and_completion(
    tmp_path: Path,
    mocker: MockerFixture,
    leak_stream: str,
    times_out: bool,
    cleanup_failure: str,
) -> None:
    # Arrange a harmless static-ELF input and a fake Docker executable. The
    # subject's real subprocess capture times out on a real hung local child.
    payload = bytearray(128)
    payload[:6] = b"\x7fELF\x02\x01"
    struct.pack_into("<H", payload, 18, 183)
    struct.pack_into("<Q", payload, 32, 64)
    struct.pack_into("<HH", payload, 54, 56, 1)
    struct.pack_into("<IIQQQQQQ", payload, 64, 1, 0, 120, 0, 0, 8, 8, 0)
    binary = tmp_path / "fixture-elf"
    binary.write_bytes(payload)
    binary.chmod(0o755)
    fake_key = "SYNTHETIC_TIMEOUT_KEY_NOT_A_CREDENTIAL"
    credential = tmp_path / "synthetic-key"
    credential.write_text(fake_key)
    credential.chmod(0o600)
    tools = tmp_path / "tools"
    tools.mkdir()
    cleanup = tmp_path / "cleanup-observed"
    fake_docker = tools / "docker"
    fake_docker.write_text(
        f"#!{sys.executable}\n"
        "import os, signal, sys\n"
        "from pathlib import Path\n"
        "if sys.argv[1] == 'image': raise SystemExit(0)\n"
        "if sys.argv[1] == 'rm':\n"
        f" Path({str(cleanup)!r}).write_text('owned cleanup')\n"
        + (
            " signal.pause()\n"
            if cleanup_failure == "timeout"
            else " raise SystemExit(0)\n"
        )
        + (
            "print('PARTIAL_STDOUT_SENTINEL', flush=True)\n"
            "print('PARTIAL_STDERR_SENTINEL', file=sys.stderr, flush=True)\n"
            if leak_stream != "empty"
            else ""
        )
        + (
            "print(os.environ['OPENROUTER_API_KEY'], "
            f"file=sys.{leak_stream}, flush=True)\n"
            if leak_stream in ("stdout", "stderr")
            else ""
        )
        + ("os.unlink(sys.argv[0])\n" if cleanup_failure == "launch" else "")
        + ("signal.pause()\n" if times_out else "raise SystemExit(1)\n")
    )
    fake_docker.chmod(0o755)
    evidence = tmp_path / "evidence"
    mocker.patch.dict(os.environ, {"PATH": f"{tools}:/usr/bin:/bin"})
    mocker.patch.object(
        sys,
        "argv",
        [
            "hosted_lint.py",
            "--evidence",
            str(evidence),
            "--binary",
            str(binary),
            "--sha256",
            hashlib.sha256(payload).hexdigest(),
            "--commit",
            "a" * 40,
            "--version",
            "0.0.6",
            "--credential-file",
            str(credential),
        ],
    )
    real_run = subprocess.run
    run_environment: dict[str, str] | None = None

    def bounded_run(
        command: list[str],
        *,
        env: dict[str, str] | None = None,
        capture_output: bool = False,
        timeout: float = 15,
        check: bool = False,
    ) -> subprocess.CompletedProcess[bytes]:
        nonlocal run_environment
        if command[:2] == ["docker", "run"]:
            run_environment = env
            timeout = 1
        elif command[:2] == ["docker", "rm"] and cleanup_failure == "timeout":
            timeout = 0.5
        return real_run(
            command,
            env=env,
            capture_output=capture_output,
            timeout=timeout,
            check=check,
        )

    calls = mocker.patch.object(hosted_lint.subprocess, "run", side_effect=bounded_run)

    # Act through the real main/capture/timeout/output policy. Only the outside
    # Docker executable and the long timeout duration are substituted.
    with pytest.raises(SystemExit) as stopped:
        hosted_lint.main()

    # Assert the output policy, unknown outcome, no retry and owned cleanup.
    assert stopped.value.code == 2
    assert run_environment is not None and "OPENROUTER_API_KEY" not in run_environment
    assert (
        sum(call.args[0][:2] == ["docker", "run"] for call in calls.call_args_list) == 1
    )
    if times_out:
        timeout_receipt = json.loads((evidence / "s3-timeout.json").read_text())
        assert timeout_receipt["remote_outcome"] == "unknown"
        assert timeout_receipt["retry_performed"] is False
        assert timeout_receipt["output_partial"] is True
        assert (
            timeout_receipt["container_cleanup_status"]
            == {
                "none": "completed",
                "timeout": "timed_out",
                "launch": "launch_failed",
            }[cleanup_failure]
        )
        assert timeout_receipt["container_cleanup_exit"] == (
            0 if cleanup_failure == "none" else None
        )
        assert (
            sum(call.args[0][:2] == ["docker", "rm"] for call in calls.call_args_list)
            == 1
        )
        if cleanup_failure == "launch":
            assert not cleanup.exists()
            assert not fake_docker.exists()
        else:
            assert cleanup.read_text() == "owned cleanup"
    else:
        assert not (evidence / "s3-timeout.json").exists()
        assert not cleanup.exists()
    stdout_name = "stdout.partial.log" if times_out else "stdout.json"
    stderr_name = "stderr.partial.log" if times_out else "stderr.log"
    if leak_stream in ("none", "empty"):
        assert (evidence / stdout_name).read_bytes() == (
            b"" if leak_stream == "empty" else b"PARTIAL_STDOUT_SENTINEL\n"
        )
        assert (evidence / stderr_name).read_bytes() == (
            b"" if leak_stream == "empty" else b"PARTIAL_STDERR_SENTINEL\n"
        )
        assert not (evidence / "credential-leak.json").exists()
    else:
        assert json.loads((evidence / "credential-leak.json").read_text()) == {
            "observed": True,
            "raw_output_retained": False,
        }
        assert not (evidence / stdout_name).exists()
        assert not (evidence / stderr_name).exists()
    assert all(
        fake_key.encode() not in file.read_bytes()
        for file in evidence.rglob("*")
        if file.is_file()
    )
