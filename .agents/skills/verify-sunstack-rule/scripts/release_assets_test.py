"""Exercise current package admission through complete synthetic release archives."""

import hashlib
import io
import json
import struct
import tarfile
from pathlib import Path

import pytest
from release_assets import TARGETS, check_release

RELEASE_VERSION = "0.0.1-rehearsal.4"


def pack_regular_member(archive: Path, name: str, payload: bytes) -> None:
    with tarfile.open(archive, "w:gz") as contents:
        member = tarfile.TarInfo(name)
        member.size = len(payload)
        contents.addfile(member, io.BytesIO(payload))


def write_release_downloads(
    directory: Path, package_name: str, package_version: str
) -> None:
    directory.mkdir()
    checksums: list[str] = []
    for target in TARGETS:
        # Minimal synthetic ELF program headers exercise the real linkage reader;
        # these fixture bytes are never presented as executable product proof.
        payload = bytearray(128)
        payload[:6] = b"\x7fELF\x02\x01"
        struct.pack_into("<H", payload, 18, 62 if target.startswith("x86_64") else 183)
        struct.pack_into("<Q", payload, 32, 64)
        struct.pack_into("<HH", payload, 54, 56, 1)
        struct.pack_into("<IIQQQQQQ", payload, 64, 1, 0, 120, 0, 0, 8, 8, 0)
        archive = directory / f"sunstack-{target}.tar.gz"
        pack_regular_member(archive, "sunstack/sunstack", bytes(payload))
        checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
        _ = Path(str(archive) + ".sha256").write_text(f"{checksum} *{archive.name}\n")
        checksums.append(checksum)
    _ = (directory / "sunstack.rb").write_text(
        f'version "{RELEASE_VERSION}"\n'
        + "\n".join(f'sha256 "{checksum}"' for checksum in checksums)
    )
    _ = (directory / "sunstack-installer.sh").write_text(
        f'APP_VERSION="{RELEASE_VERSION}"\n'
    )
    _ = (directory / "sunstack.lint.schema.json").write_text('{"type":"object"}\n')
    package = {
        "name": package_name,
        "version": package_version,
        "bin": {"sunstack": "run-sunstack.js"},
        "artifactDownloadUrls": ["https://example.invalid/release"],
    }
    pack_regular_member(
        directory / "sunstack-npm-package.tar.gz",
        "package/package.json",
        json.dumps(package).encode(),
    )


def test_preflight_accepts_scoped_package_through_real_archive_and_linkage_checks(
    tmp_path: Path,
) -> None:
    downloads = tmp_path / "downloads"
    write_release_downloads(downloads, "@sunstack/cli", RELEASE_VERSION)

    receipt = check_release(downloads, RELEASE_VERSION, tmp_path / "evidence")

    assert receipt["npm_name"] == "@sunstack/cli"
    assert receipt["npm_version"] == RELEASE_VERSION
    assert receipt["formula_version"] == receipt["installer_version"] == RELEASE_VERSION
    assert all(
        (tmp_path / "evidence" / target / "sunstack").is_file() for target in TARGETS
    )


@pytest.mark.parametrize(
    ("package_name", "package_version"),
    [
        ("sunstack", RELEASE_VERSION),
        ("@wrong/cli", RELEASE_VERSION),
        ("@sunstack/cli", "0.0.1-rehearsal.5"),
    ],
)
def test_preflight_rejects_wrong_package_or_version_with_valid_other_release_inputs(
    tmp_path: Path, package_name: str, package_version: str
) -> None:
    downloads = tmp_path / "downloads"
    write_release_downloads(downloads, package_name, package_version)

    with pytest.raises(ValueError, match="Npm metadata differs from release"):
        check_release(downloads, RELEASE_VERSION, tmp_path / "evidence")
