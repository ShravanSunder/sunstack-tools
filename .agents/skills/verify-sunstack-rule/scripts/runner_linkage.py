"""Read ELF dependencies from the shipped bytes and macOS otool OS-path policy."""

import struct
import subprocess
from pathlib import Path

from runner_contracts import JSON_OBJECT, JsonObject


def inspect_elf(data: bytes, architecture: str) -> JsonObject:
    if len(data) < 64 or data[:6] != b"\x7fELF\x02\x01":
        raise ValueError("Expected 64-bit little-endian ELF")
    expected_machine = {"amd64": 62, "arm64": 183}[architecture]
    if struct.unpack_from("<H", data, 18)[0] != expected_machine:
        raise ValueError("ELF machine differs from declared platform")
    header_offset = struct.unpack_from("<Q", data, 32)[0]
    header_size, header_count = struct.unpack_from("<HH", data, 54)
    if (
        header_count == 0
        or header_size < 56
        or header_offset + header_count * header_size > len(data)
    ):
        raise ValueError("Invalid ELF program headers")
    interpreters: list[str] = []
    needed: list[int] = []
    for index in range(header_count):
        kind, _, offset, _, _, size, _, _ = struct.unpack_from(
            "<IIQQQQQQ", data, header_offset + index * header_size
        )
        if offset + size > len(data):
            raise ValueError("ELF segment outside file")
        if kind == 3:
            interpreters.append(
                data[offset : offset + size].decode(errors="replace").rstrip("\0")
            )
        if kind == 2:
            for location in range(offset, offset + size, 16):
                if location + 16 > offset + size:
                    raise ValueError("Truncated ELF dynamic entry")
                tag, value = struct.unpack_from("<qQ", data, location)
                if tag == 0:
                    break
                if tag == 1:
                    needed.append(value)
    if interpreters or needed:
        raise ValueError(
            f"Dynamic ELF dependency: PT_INTERP={interpreters}, DT_NEEDED={needed}"
        )
    return {
        "architecture": architecture,
        "elf_machine": expected_machine,
        "pt_interp": [],
        "dt_needed": [],
        "size_bytes": len(data),
    }


def inspect_macos(binary: Path) -> JsonObject:
    output = subprocess.run(
        ["/usr/bin/otool", "-L", str(binary)],
        capture_output=True,
        text=True,
        timeout=15,
        check=True,
    ).stdout
    libraries = [
        line.strip().split(" ", 1)[0]
        for line in output.splitlines()[1:]
        if line.strip()
    ]
    if not libraries or any(
        not library.startswith(("/usr/lib/", "/System/")) for library in libraries
    ):
        raise ValueError(f"Non-OS macOS dependency: {libraries}")
    return JSON_OBJECT.validate_python({"libraries": libraries, "otool": output})
