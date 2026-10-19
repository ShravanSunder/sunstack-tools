import struct
from pathlib import Path

import pytest

from scripts.release.check_linkage import (
    check_elf,
    check_macos_dependencies,
    find_archives,
)


def elf_with_program(program_type: int, dynamic_tag: int = 0) -> bytes:
    binary = bytearray(160)
    binary[:6] = b"\x7fELF\x02\x01"
    struct.pack_into("<Q", binary, 32, 64)
    struct.pack_into("<HH", binary, 54, 56, 1)
    struct.pack_into("<IIQQQQQQ", binary, 64, program_type, 0, 128, 0, 0, 32, 0, 0)
    struct.pack_into("<qQ", binary, 128, dynamic_tag, 0)
    return bytes(binary)


def test_linux_loader_or_needed_library_is_refused_but_static_elf_is_allowed() -> None:
    check_elf(elf_with_program(1))
    check_elf(elf_with_program(2))
    with pytest.raises(ValueError, match="PT_INTERP"):
        check_elf(elf_with_program(3))
    with pytest.raises(ValueError, match="DT_NEEDED"):
        check_elf(elf_with_program(2, 1))
    with pytest.raises(ValueError, match="ELF"):
        check_elf(b"not an executable")


def test_macos_homebrew_dependency_fails_while_os_dependencies_pass() -> None:
    allowed = "sunstack:\n\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0)\n\t/System/Library/Frameworks/Security.framework/Versions/A/Security (compatibility version 1.0.0)\n"
    check_macos_dependencies(allowed)
    with pytest.raises(ValueError, match="non-OS"):
        check_macos_dependencies(
            allowed
            + "\t/opt/homebrew/opt/libgit2/lib/libgit2.dylib (compatibility version 1.9.0)\n"
        )
    with pytest.raises(ValueError, match="no macOS libraries"):
        check_macos_dependencies("sunstack:\n")


def test_each_job_checks_its_own_archive_and_cannot_pass_when_it_is_missing(
    tmp_path: Path,
) -> None:
    own = tmp_path / "sunstack-aarch64-unknown-linux-musl.tar.gz"
    own.touch()
    (tmp_path / "sunstack-aarch64-apple-darwin.tar.gz").touch()
    assert find_archives(tmp_path, ["aarch64-unknown-linux-musl"]) == [own]
    with pytest.raises(ValueError, match="requested targets"):
        _ = find_archives(tmp_path, ["x86_64-unknown-linux-musl"])
