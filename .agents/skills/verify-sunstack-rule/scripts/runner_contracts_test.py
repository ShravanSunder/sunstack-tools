"""Challenge apparatus observations without invoking or changing a product build."""

import copy
import io
import json
import struct
import subprocess
import sys
import tarfile
from pathlib import Path

import pytest
from container_matrix import IMAGES, container_command
from hosted_lint import read_credential
from lint_observations import check_report, require_diagnostic, require_format
from release_assets import read_member
from runner_contracts import (
    FixtureCase,
    JsonObject,
    as_array,
    as_object,
    compare_report,
)
from runner_fixture_cases import build_fixtures
from runner_fixture_setup import expected_finding, git_command
from runner_linkage import inspect_elf
from runner_process import RunnerTarget, binary_digest, run_case
from scenario_transport import ControlledServer


def literal_report() -> JsonObject:
    return {
        "config": "/fixture/sunstack.lint.json",
        "version": "0.1.0",
        "status": {"kind": "findings"},
        "findings": [
            {
                "rule": "docs/standard",
                "file": "docs/one.md",
                "span": {"start_line": 1, "start_column": 1},
                "severity": "error",
                "message": "Use MUST",
            }
        ],
        "counts": {"error": 1, "warn": 0},
        "incomplete": [],
        "usage": {"model_requests": 0},
        "evidence_seen": {},
    }


def test_literal_comparison_accepts_additive_fields_and_rejects_wrong_observations() -> (
    None
):
    expected = FixtureCase(
        name="literal",
        findings=[expected_finding()],
        status="findings",
        wrong_build="A finding is missing or changed",
    )
    good = literal_report()
    good["new_additive_field"] = True
    compare_report(good, expected, Path("/fixture"))
    for failure in (
        "empty",
        "severity",
        "line",
        "rule",
        "count",
        "incomplete",
        "request",
        "status",
        "config",
    ):
        report = copy.deepcopy(good)
        finding = as_object(as_array(report["findings"])[0])
        if failure == "empty":
            report["findings"] = []
        elif failure == "severity":
            finding["severity"] = "warn"
        elif failure == "line":
            as_object(finding["span"])["start_line"] = 2
        elif failure == "rule":
            finding["rule"] = "docs/other"
        elif failure == "count":
            as_object(report["counts"])["error"] = 0
        elif failure == "incomplete":
            report["incomplete"] = [{"reason": {"kind": "coverage_none"}}]
        elif failure == "request":
            as_object(report["usage"])["model_requests"] = 1
        elif failure == "status":
            report["status"] = {"kind": "clean"}
        else:
            report["config"] = "/wrong/sunstack.lint.json"
        with pytest.raises(ValueError):
            compare_report(report, expected, Path("/fixture"))


def test_judged_report_needs_values_exact_evidence_reference_and_absent_cost() -> None:
    expected = FixtureCase(
        name="judged",
        findings=[expected_finding()],
        requests=1,
        cost=False,
        wrong_build="Answer/evidence lost or absent cost fabricated",
    )
    good = literal_report()
    as_object(good["usage"]).update(model_requests=1, input_tokens=20)
    good["evidence_seen"] = {"0": {"contents": {"c1": "Known fixture"}, "evidence": {}}}
    as_object(as_array(good["findings"])[0])["judgment"] = {
        "request_index": 0,
        "answers": [
            {"question": "q1", "answer": {"type": "noul", "yes_probability": 0.1}}
        ],
    }
    compare_report(good, expected, Path("/fixture"))
    check_report(
        good,
        expected,
        Path("/fixture"),
        [{"contents": {"c1": "Known fixture"}, "evidence": {}}],
    )
    with pytest.raises(ValueError):
        check_report(
            good,
            expected,
            Path("/fixture"),
            [{"contents": {"c1": "WRONG STATE"}, "evidence": {}}],
        )
    for failure in ("refs-only", "missing-evidence", "wrong-index", "zero-cost"):
        report = copy.deepcopy(good)
        judgment = as_object(as_object(as_array(report["findings"])[0])["judgment"])
        if failure == "refs-only":
            judgment["answers"] = [{"request_index": 0, "question_name": "q1"}]
        elif failure == "missing-evidence":
            report["evidence_seen"] = {}
        elif failure == "wrong-index":
            judgment["request_index"] = 7
        else:
            as_object(report["usage"])["cost_usd"] = 0
        with pytest.raises(ValueError):
            compare_report(report, expected, Path("/fixture"))


def elf_fixture(segment_kind: int = 1, needed: bool = False) -> bytes:
    data = bytearray(256)
    data[:6] = b"\x7fELF\x02\x01"
    struct.pack_into("<H", data, 18, 183)
    struct.pack_into("<Q", data, 32, 64)
    struct.pack_into("<HH", data, 54, 56, 1)
    struct.pack_into("<IIQQQQQQ", data, 64, segment_kind, 0, 128, 0, 0, 32, 32, 0)
    if needed:
        struct.pack_into("<qQ", data, 128, 1, 5)
    return bytes(data)


def test_elf_reader_rejects_interpreter_needed_library_wrong_architecture_and_truncation() -> (
    None
):
    assert inspect_elf(elf_fixture(), "arm64")["dt_needed"] == []
    for payload, architecture in (
        (elf_fixture(3), "arm64"),
        (elf_fixture(2, True), "arm64"),
        (elf_fixture(), "amd64"),
        (elf_fixture()[:70], "arm64"),
    ):
        with pytest.raises(ValueError):
            inspect_elf(payload, architecture)


def test_container_command_enforces_read_only_offline_pins_and_minimal_environment() -> (
    None
):
    command = container_command(
        Path("/binary"),
        Path("/fixture"),
        IMAGES["arm64"]["alpine"],
        "arm64",
        "owned-case",
        ["lint", "--check"],
    )
    assert command[command.index("--network") + 1] == "none"
    assert command[command.index("--pull") + 1] == "never"
    assert "--read-only" in command
    assert sum("readonly" in item for item in command) == 2
    assert command[command.index("/usr/bin/env") + 1 :][:3] == [
        "-i",
        "PATH=/usr/bin:/bin",
        "HOME=/home/runner",
    ]
    with pytest.raises(ValueError):
        container_command(
            Path("/binary"),
            Path("/fixture"),
            "alpine:latest",
            "arm64",
            "owned-case",
            [],
        )


def test_fixture_preparation_has_expected_files_and_real_dependency_histories(
    tmp_path: Path,
) -> None:
    destination = tmp_path / "fixtures"
    manifest = build_fixtures(destination)
    assert len(manifest.cases) >= 60 and len(
        {case.name for case in manifest.cases}
    ) == len(manifest.cases)
    for case in manifest.cases:
        assert (
            FixtureCase.model_validate_json(
                (destination / case.name / "expected-results.json").read_bytes()
            )
            == case
        )
    for name, changed in (
        ("changed-rule", "rules/rules.jsonl"),
        ("changed-ruleset", "rules/set.json"),
        ("changed-config", "sunstack.lint.json"),
    ):
        root = destination / name
        assert git_command(root, "diff", "--name-only") == changed
        assert git_command(root, "diff", "--name-only", "--", "docs/one.md") == ""
    root = destination / "since-target"
    assert int(git_command(root, "rev-list", "--count", "HEAD")) == 2
    assert {case.exit_code for case in manifest.cases} == {0, 1, 2, 3, 4}


def test_hosted_credential_is_owner_only_regular_file(tmp_path: Path) -> None:
    credential = tmp_path / "synthetic-credential"
    credential.write_text("SYNTHETIC_NOT_A_SECRET\n")
    credential.chmod(0o644)
    with pytest.raises(ValueError):
        read_credential(credential)
    credential.chmod(0o600)
    assert read_credential(credential) == "SYNTHETIC_NOT_A_SECRET"


def test_archive_reader_rejects_duplicate_or_symlink_binary(tmp_path: Path) -> None:
    for kind in ("valid", "duplicate", "symlink"):
        archive = tmp_path / f"{kind}.tar.gz"
        with tarfile.open(archive, "w:gz") as contents:
            member = tarfile.TarInfo("sunstack/sunstack")
            member.size = 3
            if kind == "symlink":
                member.type = tarfile.SYMTYPE
                member.linkname = "/bin/sh"
                contents.addfile(member)
            else:
                contents.addfile(member, io.BytesIO(b"abc"))
                if kind == "duplicate":
                    contents.addfile(member, io.BytesIO(b"def"))
        if kind == "valid":
            assert read_member(archive, "sunstack") == b"abc"
        else:
            with pytest.raises(ValueError):
                read_member(archive, "sunstack")


@pytest.mark.parametrize(
    "script",
    ["lint_fixtures.py", "container_matrix.py", "hosted_lint.py", "release_assets.py"],
)
def test_driver_help_and_missing_target_fail_before_pass_file(
    script: str, tmp_path: Path
) -> None:
    path = Path(__file__).parent / script
    help_result = subprocess.run(
        [sys.executable, str(path), "--help"],
        capture_output=True,
        timeout=15,
        check=False,
    )
    assert help_result.returncode == 0 and b"--evidence" in help_result.stdout
    evidence = tmp_path / "missing-target"
    result = subprocess.run(
        [sys.executable, str(path), "--evidence", str(evidence)],
        capture_output=True,
        timeout=15,
        check=False,
    )
    assert result.returncode == 2 and not evidence.exists()
    missing = str(tmp_path / "no-such-binary")
    extras = {
        "lint_fixtures.py": ["--binary", missing, "--commit", "a" * 40],
        "container_matrix.py": [
            "--linux-amd64",
            missing,
            "--linux-arm64",
            missing,
            "--version",
            "0.1.0",
        ],
        "hosted_lint.py": [
            "--binary",
            missing,
            "--sha256",
            "b" * 64,
            "--commit",
            "a" * 40,
            "--version",
            "0.1.0",
            "--credential-file",
            str(tmp_path / "never-read"),
        ],
        "release_assets.py": [
            "--older",
            missing,
            "--newer",
            missing,
            "--older-version",
            "0.0.1",
            "--newer-version",
            "0.0.2",
        ],
    }
    actual_missing = subprocess.run(
        [sys.executable, str(path), "--evidence", str(evidence), *extras[script]],
        capture_output=True,
        timeout=15,
        check=False,
    )
    assert (
        actual_missing.returncode == 2
        and b"failed" in actual_missing.stderr
        and not evidence.exists()
    )


def test_owned_process_observation_rejects_wrong_exit_and_keeps_raw_evidence(
    tmp_path: Path,
) -> None:
    source = tmp_path / "source"
    source.mkdir()
    git_command(source, "init", "-q")
    (source / "marker.txt").write_text("Apparatus control, not a product target.\n")
    git_command(source, "add", ".")
    git_command(source, "commit", "-qm", "Harness control")
    binary = Path("/usr/bin/true")
    target = RunnerTarget(
        binary=binary,
        source=source,
        commit=git_command(source, "rev-parse", "HEAD"),
        digest=binary_digest(binary),
    )
    with ControlledServer(tmp_path / "http") as server:
        allowed = FixtureCase(
            name="allowed-exit",
            arguments=[],
            exit_code=0,
            wrong_build="Harness drops actual exit",
        )
        exit_code, _, _ = run_case(
            target, source, tmp_path / "allowed", allowed, server
        )
        assert exit_code == 0
        rejected = allowed.model_copy(update={"name": "wrong-exit", "exit_code": 1})
        with pytest.raises(ValueError, match="exit 0, expected 1"):
            run_case(target, source, tmp_path / "rejected", rejected, server)
    assert (tmp_path / "rejected/process.json").is_file()
    assert (tmp_path / "rejected/stdout.log").is_file()


def test_invalid_diagnostic_cannot_replace_coordinates_with_null_or_zero() -> None:
    case = FixtureCase(
        name="invalid-location",
        mode="check",
        exit_code=2,
        diagnostic="syntax",
        wrong_build="Diagnostic labels exist but positions are absent",
    )
    valid: JsonObject = {
        "diagnostics": [
            {"code": "syntax", "file": "sunstack.lint.json", "line": 2, "column": 3}
        ]
    }
    require_diagnostic(json.dumps(valid), case, "sunstack: ready\n")
    require_diagnostic("syntax sunstack.lint.json:2:3", case)
    for field, value in (("line", None), ("column", 0), ("file", None), ("line", True)):
        invalid = copy.deepcopy(valid)
        as_object(as_array(invalid["diagnostics"])[0])[field] = value
        with pytest.raises(ValueError, match="positive"):
            require_diagnostic(json.dumps(invalid), case)


def test_each_output_format_must_keep_the_same_finding_message() -> None:
    report = literal_report()
    root = Path("/fixture")
    for format_name, output in (
        ("default", "docs/one.md:1:1 error docs/standard Use MUST"),
        ("github", "::error file=docs/one.md,line=1,col=1::docs/standard Use MUST"),
    ):
        require_format(output, report, format_name, root)
        with pytest.raises(ValueError, match="message"):
            require_format(output.replace("Use MUST", ""), report, format_name, root)
    sarif: JsonObject = {
        "version": "2.1.0",
        "runs": [
            {
                "results": [
                    {
                        "ruleId": "docs/standard",
                        "level": "error",
                        "message": {"text": "Use MUST"},
                        "locations": [
                            {
                                "physicalLocation": {
                                    "artifactLocation": {"uri": "docs/one.md"},
                                    "region": {"startLine": 1, "startColumn": 1},
                                }
                            }
                        ],
                    }
                ]
            }
        ],
    }
    require_format(json.dumps(sarif), report, "sarif", root)
    with pytest.raises(ValueError, match="message"):
        require_format(json.dumps(sarif).replace("Use MUST", ""), report, "sarif", root)
