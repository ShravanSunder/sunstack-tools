"""Compare real lint output with independent S1 fixture expectations."""

import re
from pathlib import Path

from runner_contracts import (
    JSON_OBJECT,
    JSON_VALUE,
    FixtureCase,
    JsonObject,
    JsonValue,
    as_array,
    as_object,
    compare_report,
    report_findings,
)


def has_location(value: JsonValue) -> bool:
    if isinstance(value, dict):
        if (
            isinstance(value.get("file"), str)
            and value["file"]
            and type(value.get("line")) is int
            and type(value.get("column")) is int
            and int(str(value["line"])) > 0
            and int(str(value["column"])) > 0
        ):
            return True
        return any(has_location(child) for child in value.values())
    if isinstance(value, list):
        return any(has_location(child) for child in value)
    return False


def require_diagnostic(stdout: str, case: FixtureCase, stderr: str = "") -> None:
    output = stdout + stderr
    if case.diagnostic and case.diagnostic not in output:
        raise ValueError(f"{case.name}: diagnostic {case.diagnostic!r} is missing")
    for required in case.includes:
        if required not in output:
            raise ValueError(f"{case.name}: missing {required!r}")
    if case.mode == "check" and case.exit_code == 2:
        try:
            located = has_location(JSON_VALUE.validate_json(stdout))
        except ValueError:
            located = re.search(r":[1-9]\d*:[1-9]\d*", output) is not None
        if not located:
            raise ValueError("Invalid config/rules lost positive file/line/column")


def require_rules(output_text: str, case: FixtureCase) -> None:
    try:
        output = JSON_VALUE.validate_json(output_text)
    except ValueError:
        lowered = output_text.casefold()
        if not all(
            word in lowered
            for word in (
                "docs/standard",
                "mechanical",
                "authored",
                "configured",
                "error",
                "off",
                "rules.jsonl",
            )
        ):
            raise ValueError(
                "Rule listing lost identity/source/authored/configured fields"
            ) from None
        if case.rules_file and not all(
            word in lowered for word in ("warn", "override", "0")
        ):
            raise ValueError(
                "File-specific rule listing lost deciding override"
            ) from None
        return
    if not isinstance(output, (dict, list)):
        raise TypeError("Rule listing is not a structured or readable list")
    values = as_array(output["rules"]) if isinstance(output, dict) else output
    if len(values) != 1:
        raise ValueError("--rules must include exactly the loaded rule, including off")
    item = as_object(values[0])
    for field in ("rule", "source", "kind", "authored", "configured"):
        if field not in item:
            raise ValueError(f"Rule listing missing {field}")
    if (
        item["kind"] != "mechanical"
        or item["authored"] != "error"
        or item["configured"] != "off"
    ):
        raise ValueError("Rule listing loses authored/configured distinction")
    source = as_object(item["source"])
    if not source.get("file") or not source.get("line"):
        raise ValueError("Rule source location missing")
    if case.rules_file:
        effective = as_object(item["for_file"])
        if (
            effective["setting"] != case.rules_setting
            or effective["decided_by"] != case.rules_origin
        ):
            raise ValueError("--rules file chose the wrong setting/origin")


def require_format(
    output: str, report: JsonObject, format_name: str, root: Path
) -> None:
    expected = report_findings(report, root)
    messages = {
        str(as_object(value)["rule"]): str(as_object(value)["message"])
        for value in as_array(report["findings"])
    }
    if format_name == "sarif":
        sarif = JSON_OBJECT.validate_json(output)
        if sarif["version"] != "2.1.0":
            raise ValueError("Wrong SARIF version")
        results = as_array(as_object(as_array(sarif["runs"])[0])["results"])
        actual: list[tuple[str, str, int, int, str]] = []
        for raw in results:
            item = as_object(raw)
            location = as_object(
                as_object(as_array(item["locations"])[0])["physicalLocation"]
            )
            region = as_object(location["region"])
            if as_object(item["message"])["text"] != messages[str(item["ruleId"])]:
                raise ValueError("SARIF finding message differs from JSON")
            actual.append(
                (
                    str(item["ruleId"]),
                    str(as_object(location["artifactLocation"])["uri"]),
                    int(str(region["startLine"])),
                    int(str(region["startColumn"])),
                    "warn" if item["level"] == "warning" else str(item["level"]),
                )
            )
        wanted = [
            (finding.rule, finding.file, finding.line, finding.column, finding.severity)
            for finding in expected
        ]
        if sorted(actual) != sorted(wanted):
            raise ValueError("SARIF findings differ from JSON")
        return
    lines = [
        line
        for line in output.splitlines()
        if any(finding.rule in line for finding in expected)
    ]
    if len(lines) != len(expected):
        raise ValueError(f"{format_name}: findings missing or duplicated")
    for finding in expected:
        severity = (
            "warning"
            if format_name == "github" and finding.severity == "warn"
            else finding.severity
        )
        candidates = [
            line
            for line in lines
            if finding.rule in line and finding.file in line and severity in line
        ]
        if len(candidates) != 1:
            raise ValueError(f"{format_name}: finding identity/severity missing")
        line = candidates[0]
        if messages[finding.rule] not in line:
            raise ValueError(f"{format_name}: finding message differs from JSON")
        if format_name == "github":
            if (
                f"::{severity} " not in line
                or f"line={finding.line}" not in line
                or f"col={finding.column}" not in line
            ):
                raise ValueError("GitHub annotation location missing")
        elif not re.search(rf"(?:[:(, ]){finding.line}(?:[:,) ])", line):
            raise ValueError("Default format location missing")


def check_report(
    report: JsonObject, case: FixtureCase, root: Path, captured_states: list[JsonObject]
) -> None:
    compare_report(report, case, root)
    if case.requests and not case.cancel and not case.incomplete:
        evidence = as_object(report["evidence_seen"])
        if sorted(evidence.values(), key=str) != sorted(captured_states, key=str):
            raise ValueError(
                "Evidence seen differs from exact controlled request states"
            )
        for raw in as_array(report["findings"]):
            item = as_object(raw)
            if item.get("judgment") is not None:
                judgment = as_object(item["judgment"])
                for raw_answer in as_array(judgment["answers"]):
                    answer = as_object(as_object(raw_answer)["answer"])
                    if (
                        answer["type"] != "noul"
                        or answer["yes_probability"] != case.probability
                    ):
                        raise ValueError(
                            "Resolved typed answer differs from controlled native value"
                        )
