"""Reject non-standalone executables in the actual Sunstack release archives."""

import argparse
import re
import struct
import subprocess
import tarfile
import tempfile
from pathlib import Path

ARCHIVE_NAME = re.compile(
    r"sunstack-(aarch64|x86_64)-(apple-darwin|unknown-linux-musl)\.tar\.gz"
)
TARGETS = (
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "aarch64-unknown-linux-musl",
    "x86_64-unknown-linux-musl",
)


def check_elf(binary: bytes) -> None:
    if binary[:6] != b"\x7fELF\x02\x01":
        raise ValueError("expected a 64-bit little-endian ELF executable")
    header_offset: int = struct.unpack_from("<Q", binary, 32)[0]
    header_size, header_count = struct.unpack_from("<HH", binary, 54)
    for index in range(header_count):
        program_type, _, offset, _, _, size, _, _ = struct.unpack_from(
            "<IIQQQQQQ", binary, header_offset + index * header_size
        )
        if program_type == 3:
            raise ValueError("Linux archive contains a PT_INTERP loader")
        if program_type == 2:
            for entry in range(offset, offset + size, 16):
                tag, _ = struct.unpack_from("<qQ", binary, entry)
                if tag == 0:
                    break
                if tag == 1:
                    raise ValueError(
                        "Linux archive contains a DT_NEEDED shared library"
                    )


def check_macos_dependencies(otool_output: str) -> None:
    dependencies = [
        line.strip().split(" (", 1)[0]
        for line in otool_output.splitlines()[1:]
        if line.strip()
    ]
    if not dependencies:
        raise ValueError("otool reported no macOS libraries")
    forbidden = [
        path
        for path in dependencies
        if not path.startswith(("/usr/lib/", "/System/Library/"))
    ]
    if forbidden:
        raise ValueError(f"macOS archive contains non-OS libraries: {forbidden}")


def check_archive(archive: Path) -> None:
    with tarfile.open(archive, "r:gz") as packed:
        binaries = [
            member
            for member in packed.getmembers()
            if member.isfile() and Path(member.name).name == "sunstack"
        ]
        if len(binaries) != 1:
            raise ValueError(f"{archive}: expected exactly one sunstack executable")
        stream = packed.extractfile(binaries[0])
        if stream is None:
            raise ValueError(f"{archive}: cannot read the executable")
        binary = stream.read()
    if archive.name.endswith("linux-musl.tar.gz"):
        check_elf(binary)
    else:
        with tempfile.TemporaryDirectory(prefix="sunstack-linkage-") as directory:
            executable = Path(directory) / "sunstack"
            _ = executable.write_bytes(binary)
            inspected = subprocess.run(
                ["otool", "-L", str(executable)],
                check=True,
                capture_output=True,
                text=True,
            )
            check_macos_dependencies(inspected.stdout)
    print(f"{archive.name}: standalone linkage verified")


class LinkageArguments(argparse.Namespace):
    directory: Path = Path(".")
    targets: list[str] | None = None


def find_archives(directory: Path, targets: list[str] | None) -> list[Path]:
    archives = (
        [directory / f"sunstack-{target}.tar.gz" for target in targets]
        if targets
        else sorted(
            path for path in directory.iterdir() if ARCHIVE_NAME.fullmatch(path.name)
        )
    )
    if not archives or any(not archive.is_file() for archive in archives):
        raise ValueError(
            "no Sunstack binary release archives found for the requested targets"
        )
    return archives


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    _ = parser.add_argument(
        "directory", type=Path, help="cargo-dist artifact directory"
    )
    _ = parser.add_argument(
        "--target",
        dest="targets",
        action="append",
        choices=TARGETS,
        help="check only this build job's target; repeat for multiple targets",
    )
    arguments = parser.parse_args(namespace=LinkageArguments())
    directory: Path = arguments.directory
    for archive in find_archives(directory, arguments.targets):
        check_archive(archive)


if __name__ == "__main__":
    main()
