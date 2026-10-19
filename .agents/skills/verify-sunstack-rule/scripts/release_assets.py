# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic==2.14.1", "pydantic_core==2.50.1"]
# ///
"""S4 artifact preflight: verify two authenticated release downloads locally.

--older <directory> --older-version <version-without-v> --newer <directory>
--newer-version <version> --evidence <new-dir>. Requires each tag's four .tar.gz
archives and .sha256 files, shell installer, formula, npm package and schema.
Reads archives without extracting untrusted paths; retains only sunstack bytes
in its own evidence directory. No install, network, tags or repository writes.
This is artifact integrity proof, never the complete release rehearsal.
"""

import argparse
import hashlib
import re
import tarfile
from pathlib import Path

from runner_contracts import JSON_OBJECT, JsonObject, write_json
from runner_linkage import inspect_elf

TARGETS = (
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
)


def read_member(archive: Path, leaf: str) -> bytes:
    with tarfile.open(archive, "r:gz") as contents:
        matches = [member for member in contents if Path(member.name).name == leaf]
        if (
            len(matches) != 1
            or not matches[0].isfile()
            or matches[0].size > 256 * 1024 * 1024
        ):
            raise ValueError(f"Archive needs exactly one bounded regular {leaf}")
        stream = contents.extractfile(matches[0])
        if stream is None:
            raise ValueError("Archive member is unreadable")
        return stream.read()


def check_release(directory: Path, version: str, evidence: Path) -> JsonObject:
    evidence.mkdir(parents=True, exist_ok=False)
    digests: JsonObject = {}
    binary_digests: JsonObject = {}
    for target in TARGETS:
        name = f"sunstack-{target}.tar.gz"
        archive = directory / name
        payload = archive.read_bytes()
        checksum = (directory / f"{name}.sha256").read_text().split()[0]
        if (
            not re.fullmatch("[0-9a-fA-F]{64}", checksum)
            or hashlib.sha256(payload).hexdigest() != checksum.lower()
        ):
            raise ValueError(f"Release checksum mismatch: {name}")
        digests[target] = checksum.lower()
        binary = read_member(archive, "sunstack")
        binary_digests[target] = hashlib.sha256(binary).hexdigest()
        target_directory = evidence / target
        target_directory.mkdir()
        (target_directory / "sunstack").write_bytes(binary)
        (target_directory / "sunstack").chmod(0o755)
        if target.endswith("linux-musl"):
            inspect_elf(binary, "amd64" if target.startswith("x86_64") else "arm64")
    formula = (directory / "sunstack.rb").read_text()
    if f'version "{version}"' not in formula or any(
        str(value) not in formula for value in digests.values()
    ):
        raise ValueError("Formula version or platform checksums differ from release")
    installer = (directory / "sunstack-installer.sh").read_text()
    if f'APP_VERSION="{version}"' not in installer:
        raise ValueError("Shell installer version differs from release")
    package = JSON_OBJECT.validate_json(
        read_member(directory / "sunstack-npm-package.tar.gz", "package.json")
    )
    if package["name"] != "sunstack" or package["version"] != version:
        raise ValueError("Npm metadata differs from release")
    if not package["artifactDownloadUrls"]:
        raise ValueError("Npm release is in demo mode")
    schema = JSON_OBJECT.validate_json(
        (directory / "sunstack.lint.schema.json").read_bytes()
    )
    if not schema:
        raise ValueError("Released config schema is empty")
    receipt: JsonObject = {
        "version": version,
        "archive_sha256": digests,
        "binary_sha256": binary_digests,
        "npm_name": package["name"],
        "npm_version": package["version"],
        "formula_version": version,
        "installer_version": version,
    }
    write_json(evidence / "artifacts.json", receipt)
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--older", type=Path, required=True)
    parser.add_argument("--older-version", required=True)
    parser.add_argument("--newer", type=Path, required=True)
    parser.add_argument("--newer-version", required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    arguments = parser.parse_args()
    if arguments.older_version == arguments.newer_version:
        parser.error("Two distinct release versions are required")
    try:
        if not arguments.older.is_dir() or not arguments.newer.is_dir():
            raise ValueError(
                "Both authenticated release download directories must exist"
            )
        evidence = arguments.evidence.resolve()
        evidence.mkdir(parents=True, exist_ok=False)
        older = check_release(
            arguments.older, arguments.older_version, evidence / "older"
        )
        newer = check_release(
            arguments.newer, arguments.newer_version, evidence / "newer"
        )
        if older["binary_sha256"] == newer["binary_sha256"]:
            raise ValueError(
                "Both release tags contain identical bytes; version pin proof would be vacuous"
            )
        write_json(
            evidence / "s4-artifacts.json",
            {
                "older": older,
                "newer": newer,
                "installation_observed": False,
                "gaps": [
                    "Homebrew/shell/npm installations",
                    "older-version shell request",
                    "fresh minimal-environment macOS/Linux",
                    "npm report/exit/Ctrl-C and child settlement",
                    "public channels outside scope",
                ],
            },
        )
        print(
            "Two release artifact sets verified; installation rehearsal remains unobserved"
        )
    except (ValueError, TypeError, OSError, tarfile.TarError) as error:
        parser.exit(2, f"S4 artifact setup/observation failed: {error}\n")


if __name__ == "__main__":
    main()
