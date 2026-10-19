"""Exercise the owned npm entrypoint against a real child process and Unix signals."""

import json
import os
import select
import shutil
import signal
import subprocess
import sys
from pathlib import Path

import pytest

LAUNCHER = Path(__file__).with_name("npm-launcher") / "run.js"
CHILD_SOURCE = """import json, os, signal, sys
if sys.argv[1] == "exit":
    print("exact stdout", flush=True)
    print("exact stderr", file=sys.stderr, flush=True)
    raise SystemExit(int(sys.argv[2]))
def cancelled(signum, frame):
    signal.signal(signal.SIGINT, signal.SIG_IGN)
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    print(json.dumps({"status": "cancelled", "exit": 4}), flush=True)
    os._exit(4)
signal.signal(signal.SIGINT, cancelled)
signal.signal(signal.SIGTERM, cancelled)
print("READY " + str(os.getpid()), flush=True)
signal.pause()
"""


@pytest.fixture
def installed_launcher(tmp_path: Path) -> tuple[str, Path, Path]:
    node = shutil.which("node")
    assert node is not None, "the npm launcher tests require Node"
    launcher = tmp_path / "run.cjs"
    _ = shutil.copyfile(LAUNCHER, launcher)
    child = tmp_path / "child.py"
    _ = child.write_text(CHILD_SOURCE)
    # Platform/download selection is outside this test's process-lifetime boundary.
    # The production package path and real native CLI are covered by dist proof.
    _ = (tmp_path / "binary.js").write_text(
        "exports.getPackage = () => ({exists: () => true, "
        + "install: () => {throw Error('unexpected install')}, "
        + f"installDirectory: {json.dumps(str(Path(sys.executable).parent))}, "
        + f"binaries: {{sunstack: {json.dumps(Path(sys.executable).name)}}}}});"
    )
    return node, launcher, child


@pytest.mark.parametrize("exit_code", [0, 1, 2, 3])
def test_native_exit_and_both_streams_pass_through_unchanged(
    installed_launcher: tuple[str, Path, Path], exit_code: int
) -> None:
    node, launcher, child = installed_launcher
    result = subprocess.run(
        [node, str(launcher), str(child), "exit", str(exit_code)],
        capture_output=True,
        check=False,
        timeout=10,
    )
    assert result.returncode == exit_code
    assert result.stdout == b"exact stdout\n"
    assert result.stderr == b"exact stderr\n"


@pytest.mark.parametrize(
    ("sent_signal", "whole_group", "expected_exit"),
    [
        (signal.SIGINT, True, 4),
        (signal.SIGINT, False, 4),
        (signal.SIGTERM, False, 4),
        (signal.SIGHUP, False, -signal.SIGHUP),
    ],
)
def test_signals_reach_child_and_launcher_waits_for_its_outcome(
    installed_launcher: tuple[str, Path, Path],
    sent_signal: signal.Signals,
    whole_group: bool,
    expected_exit: int,
) -> None:
    node, launcher, child = installed_launcher
    process: subprocess.Popen[bytes] = subprocess.Popen(
        [node, str(launcher), str(child), "signal"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        start_new_session=True,
    )
    child_pid: int | None = None
    try:
        assert process.stdout is not None
        readable, _, _ = select.select([process.stdout], [], [], 10)
        assert readable, "real child did not declare signal readiness"
        ready = os.read(process.stdout.fileno(), 64)
        assert ready.startswith(b"READY ")
        child_pid = int(ready.split()[1])
        if whole_group:
            os.killpg(process.pid, sent_signal)
        else:
            process.send_signal(sent_signal)
        stdout, stderr = process.communicate(timeout=10)
        assert process.returncode == expected_exit
        if sent_signal != signal.SIGHUP:
            assert json.loads(stdout) == {"status": "cancelled", "exit": 4}
        else:
            assert stdout == b""
        assert stderr == b""
        with pytest.raises(ProcessLookupError):
            os.kill(child_pid, 0)
    finally:
        if child_pid is not None:
            try:
                os.kill(child_pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        if process.poll() is None:
            process.kill()
        _ = process.communicate(timeout=10)
