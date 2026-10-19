"""Independent runner fixture expectations and stable report observations."""

import json
import typing as t
from pathlib import Path

from pydantic import BaseModel, ConfigDict, Field, JsonValue, TypeAdapter

type JsonObject = dict[str, JsonValue]
JSON_OBJECT = TypeAdapter(JsonObject)
JSON_VALUE = TypeAdapter(JsonValue)
type Severity = t.Literal["error", "warn"]


class FixtureFinding(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True, strict=True)
    rule: str
    file: str
    line: int = 1
    column: int = 1
    severity: Severity
    kind: t.Literal["mechanical", "judged"] = "mechanical"


class FixtureCase(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True, strict=True)
    name: str
    arguments: list[str] = Field(
        default_factory=lambda: ["lint", "--all", "--format", "json"]
    )
    cwd: str = "."
    exit_code: int = 1
    findings: list[FixtureFinding] = Field(default_factory=list)
    status: str | None = None
    mode: t.Literal["report", "check", "rules", "init", "text"] = "report"
    requests: int = 0
    probability: float = 0.1
    credential: bool = False
    cancel: bool = False
    cost: bool = True
    incomplete: bool = False
    diagnostic: str | None = None
    includes: list[str] = Field(default_factory=list)
    exclusion: str | None = None
    widened: bool = False
    selected_config: str = "sunstack.lint.json"
    unreadable: str | None = None
    rules_file: str | None = None
    rules_setting: str | None = None
    rules_rule: str | None = None
    rules_source_file: str | None = None
    rules_source_line: int | None = None
    rules_origin: JsonValue = None
    wrong_build: str
    source: str = "runner Specification R1–R15; Verification Design S1"


class FixtureManifest(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True, strict=True)
    oracle: str = "38d5f2d18b12ce1df87a83270f545976bfc707df"
    cases: list[FixtureCase]
    gaps: list[str]


def read_object(path: Path) -> JsonObject:
    return JSON_OBJECT.validate_json(path.read_bytes())


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(JSON_VALUE.validate_python(value), indent=2) + "\n")


def as_object(value: JsonValue) -> JsonObject:
    if not isinstance(value, dict):
        raise TypeError("Expected JSON object")
    return value


def as_array(value: JsonValue) -> list[JsonValue]:
    if not isinstance(value, list):
        raise TypeError("Expected JSON array")
    return value


def selector_name(value: JsonValue) -> str:
    if not isinstance(value, str) or value.count("/") != 1:
        raise ValueError("Finding rule must be its written entry/rule selector")
    return value


def report_findings(report: JsonObject, root: Path) -> list[FixtureFinding]:
    findings: list[FixtureFinding] = []
    for value in as_array(report["findings"]):
        item = as_object(value)
        file_value = item["file"]
        if not isinstance(file_value, str):
            raise TypeError("Finding file is not text")
        file_path = Path(file_value)
        if file_path.is_absolute():
            file_value = file_path.relative_to(root).as_posix()
        span = as_object(item["span"])
        if not isinstance(item["message"], str) or not item["message"]:
            raise ValueError("Finding message is empty")
        findings.append(
            FixtureFinding.model_validate(
                {
                    "rule": selector_name(item["rule"]),
                    "file": file_value,
                    "line": span["start_line"],
                    "column": span["start_column"],
                    "severity": item["severity"],
                    "kind": "judged"
                    if item.get("judgment") is not None
                    else "mechanical",
                }
            )
        )
    return sorted(
        findings,
        key=lambda finding: (
            finding.rule,
            finding.file,
            finding.line,
            finding.column,
            finding.severity,
        ),
    )


def compare_report(report: JsonObject, expected: FixtureCase, root: Path) -> None:
    actual = report_findings(report, root)
    wanted = sorted(
        expected.findings,
        key=lambda finding: (
            finding.rule,
            finding.file,
            finding.line,
            finding.column,
            finding.severity,
        ),
    )
    if actual != wanted:
        raise ValueError(f"{expected.name}: findings differ: {actual!r} != {wanted!r}")
    counts = as_object(report["counts"])
    for severity in ("error", "warn"):
        if type(counts[severity]) is not int or counts[severity] != sum(
            finding.severity == severity for finding in wanted
        ):
            raise ValueError(f"{expected.name}: severity count mismatch")
    status_name = as_object(report["status"])["kind"]
    if expected.status and status_name != expected.status:
        raise ValueError(
            f"{expected.name}: status {status_name!r} != {expected.status}"
        )
    if bool(as_array(report["incomplete"])) != expected.incomplete:
        raise ValueError(f"{expected.name}: incomplete parts mismatch")
    usage = as_object(report["usage"])
    if (
        type(usage["model_requests"]) is not int
        or usage["model_requests"] != expected.requests
    ):
        raise ValueError(f"{expected.name}: request count mismatch")
    if expected.requests and not expected.cancel and not expected.incomplete:
        evidence = as_object(report["evidence_seen"])
        if len(evidence) != expected.requests:
            raise ValueError("Request evidence is missing")
        if expected.cost and not isinstance(usage.get("cost_usd"), (int, float)):
            raise ValueError("Reported cost was dropped")
        if not expected.cost and usage.get("cost_usd") is not None:
            raise ValueError("Missing cost fabricated as zero")
    judged_rules = {finding.rule for finding in wanted if finding.kind == "judged"}
    for raw in as_array(report["findings"]):
        item = as_object(raw)
        if selector_name(item["rule"]) not in judged_rules:
            continue
        payload = item.get("judgment")
        if payload is None:
            raise ValueError("Expected judged finding lost its resolved judgment")
        judgment = as_object(payload)
        answers_value = judgment.get("answers")
        if answers_value is None:
            raise ValueError("Expected judged finding lost its resolved answers")
        answers = as_array(answers_value)
        if not answers:
            raise ValueError("Expected judged finding lost its resolved answers")
        for raw_answer in answers:
            answer = as_object(raw_answer).get("answer")
            if answer is None:
                raise ValueError("Typed answers were replaced by references or null")
            if not isinstance(as_object(answer).get("type"), str):
                raise TypeError("Resolved answer lacks its native type")
        request_index = judgment.get("request_index")
        if type(request_index) is not int or request_index < 0:
            raise ValueError("Judgment lost its numeric request index")
        if str(request_index) not in as_object(report["evidence_seen"]):
            raise ValueError("Judgment does not point at request evidence")
    if expected.exclusion and as_object(report["exclusions"])[expected.exclusion] == 0:
        raise ValueError(f"Missing {expected.exclusion} exclusion")
    if expected.widened and not as_object(report["scope"]).get("widened"):
        raise ValueError("Changed config did not record widened scope")
    config_path = report["config"]
    if (
        not isinstance(config_path, str)
        or (
            Path(config_path) if Path(config_path).is_absolute() else root / config_path
        ).resolve()
        != (root / expected.selected_config).resolve()
    ):
        raise ValueError("Wrong governing config")
