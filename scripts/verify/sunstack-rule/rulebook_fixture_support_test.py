"""Challenge fixture preservation, comparators and actual child/HTTP apparatus boundaries."""

import copy
import json
import sys
from pathlib import Path

import pytest
from lint_runner_rulebook import LintCase, require_report
from pydantic import JsonValue, ValidationError
from rulebook_check_refusals import prepare_cases
from rulebook_evidence_checks import listing_pieces
from rulebook_fixture_support import (
    CONFIG,
    SKILL,
    JsonObject,
    baseline_git,
    git_command,
    read_object,
)
from rulebook_process import RulebookProcess, declared_target


def error_report() -> JsonObject:
    return {
        "status": {"kind": "findings"},
        "findings": [
            {
                "marker": "K002",
                "file": SKILL,
                "line": 3,
                "severity": "error",
                "message": "Over 300 characters.",
            }
        ],
        "usage": {"model_requests": 0},
        "counts": {"error": 1, "warn": 0},
    }


@pytest.mark.parametrize(
    "field,value",
    [
        ("marker", "K003"),
        ("file", "wrong.md"),
        ("severity", "warn"),
        ("line", 4),
        ("message", ""),
    ],
)
def test_comparator_rejects_a_wrong_finding_identity(
    field: str, value: JsonValue
) -> None:
    # Arrange an independent, literal public report: correct exit alone is not enough.
    case = LintCase(
        name="mechanical-error", exit=1, status="findings", severity="error"
    )
    report = error_report()
    require_report(report, case)
    changed = copy.deepcopy(report)
    rows = changed["findings"]
    assert isinstance(rows, list) and isinstance(rows[0], dict)
    rows[0][field] = value
    # Act/Assert: losing the correct field must fail the same comparator.
    with pytest.raises(ValueError):
        require_report(changed, case)


def test_refusal_setup_keeps_duplicate_and_malformed_inputs(tmp_path: Path) -> None:
    # Arrange/Act without a product binary or credential.
    root = tmp_path / "cases"
    cases = prepare_cases(root)
    # Assert these cases cannot disappear through a JSON parse/rewrite in setup.
    duplicate = (root / "config-duplicate" / CONFIG).read_text()
    assert duplicate.count('"marker_prefix"') == 2
    with pytest.raises(json.JSONDecodeError):
        json.loads((root / "config-syntax" / CONFIG).read_text())
    assert len({case.name for case in cases}) == len(cases)
    assert {
        "regex-target",
        "exists-text",
        "as-kind-has-target",
        "threshold-equal",
        "probability-zero",
    }.issubset({case.name for case in cases})


def test_missing_key_compares_logical_acquisitions_not_physical_reads() -> None:
    case = LintCase(name="missing-key", exit=2, status="invalid", marker="K003")
    report: JsonObject = {
        "status": {"kind": "invalid"},
        "findings": [],
        "counts": {"error": 0, "warn": 0},
        "usage": {"model_requests": 0, "checked_files_acquired": 0},
    }
    require_report(report, case)
    report["usage"] = {"model_requests": 0, "checked_files_acquired": 1}
    with pytest.raises(ValueError, match="acquired checked content"):
        require_report(report, case)


def test_listing_reader_requires_the_declared_envelope() -> None:
    # Empty picked pieces are valid; missing pieces and the old bare-array
    # assumption are not the source-declared CLI envelope.
    raw = {
        "version": "0.0.0",
        "config": "fixture.json",
        "scope": {"kind": "all"},
        "usage": {"model_requests": 0},
        "pieces": [],
    }
    assert listing_pieces(json.dumps(raw).encode()) == []
    with pytest.raises(ValidationError):
        listing_pieces(b"[]")
    raw.pop("pieces")
    with pytest.raises(ValidationError):
        listing_pieces(json.dumps(raw).encode())


def test_real_child_uses_owned_loopback_and_keeps_actual_capture(
    tmp_path: Path,
) -> None:
    # The subject here is verification apparatus, not a stand-in product pass.
    source = tmp_path / "source"
    source.mkdir()
    binary = source / "fake-cli"
    binary.write_text(f"""#!{sys.executable}
import json, sys, urllib.request
body = {{"model": "fixture", "questions": {{"names_trigger": {{"type": "noul", "instructions": "Observe"}}}}, "state": {{"standard": "literal"}}}}
request = urllib.request.Request(sys.argv[1], data=json.dumps(body).encode(), headers={{"Content-Type": "application/json"}})
with urllib.request.urlopen(request, timeout=5) as response:
    answer = json.load(response)
print(json.dumps(answer))
""")
    binary.chmod(0o755)
    baseline_git(source)
    commit = git_command(source, "rev-parse", "HEAD")
    target = declared_target(binary, source, commit)
    with RulebookProcess(target=target, evidence=tmp_path / "evidence") as process:
        out, _, captures = process.run(
            source, "http-child", [process.server.endpoint], expected_exit=0
        )
        assert len(captures) == 1
        assert captures[0].body["state"] == {"standard": "literal"}
        assert json.loads(out)["answers"]["names_trigger"]["noul"] == 0.91
    assert (
        read_object(tmp_path / "evidence/http-child/process.json")["http_requests"] == 1
    )
    assert read_object(tmp_path / "evidence/harness.json")["errors"] == []


def test_target_check_rejects_changed_executable_bytes(tmp_path: Path) -> None:
    source = tmp_path / "source"
    source.mkdir()
    binary = source / "fake-cli"
    binary.write_text(f"#!{sys.executable}\nprint('ready')\n")
    binary.chmod(0o755)
    baseline_git(source)
    target = declared_target(binary, source, git_command(source, "rev-parse", "HEAD"))
    binary.write_text(f"#!{sys.executable}\nprint('wrong target')\n")
    with pytest.raises(ValueError, match="binary bytes changed"):
        target.confirm()
