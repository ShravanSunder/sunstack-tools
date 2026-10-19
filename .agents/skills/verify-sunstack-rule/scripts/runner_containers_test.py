"""Challenge the lasting container boundary independently of product code."""

import os
import shutil
import subprocess
import uuid
from pathlib import Path

import pytest
from runner_containers import IMAGES, ContainerRuntime, docker_client_environment
from runner_contracts import FixtureCase
from scenario_transport import ControlledServer


def test_wrong_or_mutable_image_and_non_hostname_control_routes_are_refused() -> None:
    adopted = IMAGES["arm64"]["alpine"]
    runtime = ContainerRuntime(image=adopted, control_host="host.docker.internal")
    assert runtime.architecture == "arm64"
    assert (
        runtime.control_endpoint("http://127.0.0.1:4567/systemone")
        == "http://host.docker.internal:4567/systemone"
    )
    for image in (
        "alpine:latest",
        "alpine@sha256:" + "0" * 64,
        IMAGES["arm64"]["debian"],
    ):
        with pytest.raises(ValueError, match="pinned Alpine"):
            ContainerRuntime(image=image, control_host="host.docker.internal")
    with pytest.raises(ValueError, match="hostname"):
        ContainerRuntime(image=adopted, control_host="https://example.invalid/key")
    with pytest.raises(ValueError, match="owned loopback"):
        runtime.control_endpoint("https://example.invalid/systemone")
    assert "OPENROUTER_API_KEY" not in docker_client_environment()


def test_init_alone_has_a_writable_fixture_and_other_commands_keep_it_readonly(
    tmp_path: Path,
) -> None:
    runtime = ContainerRuntime(
        image=IMAGES["arm64"]["alpine"], control_host="host.docker.internal"
    )
    normal = FixtureCase(name="readonly", wrong_build="Lint writes its inputs")
    init = normal.model_copy(update={"mode": "init"})
    for case, readonly in ((normal, True), (init, False)):
        command = runtime.command(
            tmp_path / "binary", tmp_path / "fixture", case, "owned-name"
        )
        mount = next(item for item in command if "dst=/fixture" in item)
        assert mount.endswith(",readonly") == readonly
        assert "--read-only" in command
        assert command[command.index("--network") + 1] == "none"


def test_no_call_container_cannot_reach_a_control_that_enabled_peer_reaches(
    tmp_path: Path,
) -> None:
    control_host = os.environ.get("SUNSTACK_VERIFY_CONTROL_HOST")
    if not control_host or not shutil.which("docker"):
        pytest.skip(
            "Needs a declared Docker-to-loopback route and cached pinned Alpine"
        )
    runtime = ContainerRuntime(
        image=IMAGES["arm64"]["alpine"], control_host=control_host
    )
    runtime.confirm_image()
    fixture = tmp_path / "fixture"
    fixture.mkdir()
    binary = tmp_path / "attempt-network.sh"
    with ControlledServer(tmp_path / "http") as server:
        endpoint = runtime.control_endpoint(server.endpoint)
        binary.write_text(
            "#!/bin/sh\nexec /bin/busybox wget -T 2 -q -O - --post-data='{}' '"
            + endpoint
            + "'\n"
        )
        binary.chmod(0o755)
        enabled = FixtureCase(
            name="network-control",
            arguments=[],
            requests=1,
            wrong_build="Control route cannot observe an attempted call",
        )
        positive_name = f"sunstack-test-{uuid.uuid4().hex}"
        command = runtime.command(binary, fixture, enabled, positive_name)
        process = subprocess.Popen(
            command,
            env=docker_client_environment(),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        try:
            pending = server.next()
            pending.respond(200, {"control": "reached"})
            stdout, stderr = process.communicate(timeout=10)
            assert process.returncode == 0, stderr.decode()
            assert b"reached" in stdout
            assert server.count == 1
        finally:
            if process.poll() is None:
                subprocess.run(
                    ["docker", "rm", "--force", positive_name],
                    check=False,
                    capture_output=True,
                    timeout=15,
                )
                process.wait(timeout=5)
        no_call = enabled.model_copy(update={"requests": 0})
        negative_name = f"sunstack-test-{uuid.uuid4().hex}"
        command = runtime.command(binary, fixture, no_call, negative_name)
        try:
            blocked = subprocess.run(
                command,
                check=False,
                env=docker_client_environment(),
                capture_output=True,
                timeout=10,
            )
        except subprocess.TimeoutExpired:
            subprocess.run(
                ["docker", "rm", "--force", negative_name],
                check=False,
                capture_output=True,
                timeout=15,
            )
            raise
        (tmp_path / "blocked.stderr.log").write_bytes(blocked.stderr)
        assert blocked.returncode != 0
        assert server.count == 1
