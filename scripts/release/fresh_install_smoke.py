"""Install actual release bytes from a local mirror and run with an empty environment."""

import argparse
import functools
import hashlib
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import threading
import typing as t
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


class SmokeArguments(argparse.Namespace):
    tag: str = ""
    repository: str | None = None
    assets: Path | None = None


class QuietMirror(SimpleHTTPRequestHandler):
    @t.override
    def log_message(self, format: str, *args: object) -> None:
        return


def verify_install(assets: Path, tag: str, scratch: Path) -> None:
    archives = list(assets.glob("sunstack-*-apple-darwin.tar.gz")) + list(
        assets.glob("sunstack-*-unknown-linux-musl.tar.gz")
    )
    if len(archives) != 4:
        raise ValueError("expected all four downloaded binary release archives")
    for archive in archives:
        expected_digest = Path(str(archive) + ".sha256").read_text().split()[0]
        with archive.open("rb") as stream:
            actual_digest = hashlib.file_digest(stream, "sha256").hexdigest()
        if (
            re.fullmatch(r"[0-9a-f]{64}", expected_digest) is None
            or actual_digest != expected_digest
        ):
            raise ValueError(f"release checksum mismatch: {archive.name}")
    print("All four downloaded binary archive checksums match.")
    workspace = Path(__file__).resolve().parents[2]
    examples = workspace / "docs/specs/2026-10-12-rust-rule-engine/examples"
    fixture = scratch / "fixture"
    fixture.mkdir()
    for name in [
        "mechanical.ruleset.json",
        "mechanical.rules.json",
        "mechanical-standards.md",
        "mechanical-skill.md",
    ]:
        _ = shutil.copy2(examples / name, fixture / name)
    _ = shutil.copy2(
        workspace / "scripts/release/smoke-config.json", fixture / "sunstack.lint.json"
    )
    install_home = scratch / "install-home"
    runtime_home = scratch / "empty-home"
    installed = scratch / "installed"
    install_home.mkdir()
    runtime_home.mkdir()
    mirror = ThreadingHTTPServer(
        ("127.0.0.1", 0), functools.partial(QuietMirror, directory=str(assets))
    )
    serving = threading.Thread(target=mirror.serve_forever, daemon=True)
    serving.start()
    try:
        # The bound server socket establishes readiness; no timing sleep is needed.
        installer = subprocess.run(
            ["/bin/sh", str(assets / "sunstack-installer.sh")],
            env={
                "PATH": "/usr/bin:/bin",
                "HOME": str(install_home),
                "SUNSTACK_DOWNLOAD_URL": f"http://127.0.0.1:{mirror.server_port}",
                "SUNSTACK_UNMANAGED_INSTALL": str(installed),
            },
            check=True,
            capture_output=True,
            text=True,
            timeout=120,
        )
        print(installer.stdout, end="")
    finally:
        mirror.shutdown()
        mirror.server_close()
        serving.join(timeout=10)
    binary = installed / "sunstack"
    launches: list[tuple[str, list[str]]] = [("installed", [str(binary)])]
    if sys.platform == "darwin":
        with tarfile.open(
            assets / "sunstack-x86_64-apple-darwin.tar.gz", "r:gz"
        ) as packed:
            members = [
                member
                for member in packed.getmembers()
                if member.isfile() and Path(member.name).name == "sunstack"
            ]
            if len(members) != 1:
                raise ValueError("expected exactly one macOS x64 executable")
            stream = packed.extractfile(members[0])
            if stream is None:
                raise ValueError("cannot read the macOS x64 executable")
            rosetta = scratch / "sunstack-x64"
            _ = rosetta.write_bytes(stream.read())
            rosetta.chmod(0o755)
        launches.append(("rosetta-x64", ["/usr/bin/arch", "-x86_64", str(rosetta)]))
    cases: list[tuple[list[str], int, str]] = [
        (["--version"], 0, f"sunstack {tag.removeprefix('v')}\n"),
        (["lint", "--check"], 0, "Config and rule sets are valid."),
        (["lint", "--all"], 0, "Clean: 0 error(s), 0 warning(s)"),
    ]
    for label, launch in launches:
        for arguments, expected_exit, marker in cases:
            result = subprocess.run(
                [
                    "/usr/bin/env",
                    "-i",
                    "PATH=/usr/bin:/bin",
                    f"HOME={runtime_home}",
                    *launch,
                    *arguments,
                ],
                cwd=fixture,
                check=False,
                capture_output=True,
                text=True,
                timeout=30,
            )
            if result.returncode != expected_exit or marker not in result.stdout:
                raise RuntimeError(
                    f"{label} {arguments}: exit {result.returncode}; {result.stdout}; {result.stderr}"
                )
            print(f"{label} env -i {' '.join(arguments)}: exit {result.returncode}")
    target = fixture / "mechanical-skill.md"
    original = target.read_text()
    expected = "description: Use when demonstrating local skill checks."
    if expected not in original:
        raise ValueError("the positive-control fixture changed")
    _ = target.write_text(original.replace(expected, "description: Wrong opening."))
    for label, launch in launches:
        failed = subprocess.run(
            [
                "/usr/bin/env",
                "-i",
                "PATH=/usr/bin:/bin",
                f"HOME={runtime_home}",
                *launch,
                "lint",
                "--all",
            ],
            cwd=fixture,
            check=False,
            capture_output=True,
            text=True,
            timeout=30,
        )
        if (
            failed.returncode != 1
            or "mechanical/skill.description" not in failed.stdout
        ):
            raise RuntimeError(
                f"{label} negative lint control: exit {failed.returncode}; {failed.stdout}; {failed.stderr}"
            )
        print(f"{label} env -i negative lint control: exit 1")
    if any(runtime_home.iterdir()):
        raise RuntimeError("runtime wrote into the empty HOME")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    _ = parser.add_argument("--tag", required=True)
    _ = parser.add_argument("--repository", default=os.environ.get("GITHUB_REPOSITORY"))
    _ = parser.add_argument(
        "--assets", type=Path, help="use previously downloaded real release artifacts"
    )
    arguments = parser.parse_args(namespace=SmokeArguments())
    if re.fullmatch(r"v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", arguments.tag) is None:
        raise ValueError("expected a version tag")
    with tempfile.TemporaryDirectory(prefix="sunstack-fresh-install-") as temporary:
        scratch = Path(temporary)
        if arguments.assets is None:
            if arguments.repository is None:
                raise ValueError("--repository is required when downloading artifacts")
            assets = scratch / "assets"
            assets.mkdir()
            _ = subprocess.run(
                [
                    "gh",
                    "release",
                    "download",
                    arguments.tag,
                    "--repo",
                    arguments.repository,
                    "--dir",
                    str(assets),
                ],
                check=True,
            )
        else:
            assets = arguments.assets.resolve()
        verify_install(assets, arguments.tag, scratch)


if __name__ == "__main__":
    main()
