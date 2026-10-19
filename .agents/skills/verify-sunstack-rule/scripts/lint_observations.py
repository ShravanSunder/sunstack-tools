"""Compare complete S1 records, preserving exact values across public renderings."""

import re
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import unquote

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
    selector_name,
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


def normalized_file(value: JsonValue, root: Path | None) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError("Expected a nonempty source file")
    path = Path(value)
    if path.is_absolute():
        if root is None:
            raise ValueError("Absolute source file requires its declared fixture root")
        path = path.relative_to(root)
    return path.as_posix()


def enum_name(value: JsonValue) -> str:
    if isinstance(value, str):
        return value
    name = as_object(value).get("kind")
    if not isinstance(name, str):
        raise TypeError("Expected a named setting or rule kind")
    return name


def origin_value(value: JsonValue) -> tuple[str, int | None]:
    origin = as_object(value)
    name = enum_name(origin)
    if name != "override":
        return name, None
    index = origin.get("index")
    if type(index) is not int or index < 0:
        raise ValueError("Override origin requires a nonnegative integer index")
    return name, index


def text_field(text: str, name: str) -> str:
    matches = re.findall(rf"(?:^|\s){name}\s*(?:=|:|\s)\s*([^\s,;]+)", text)
    if len(matches) != 1:
        raise ValueError(f"Rule listing requires one {name} value")
    return matches[0]


def readable_rules(output: str, for_file: bool) -> list[JsonObject]:
    records: list[JsonObject] = []
    for line in output.splitlines():
        head = re.match(r"^\s*(\S+/\S+)\s+(.+)$", line)
        if head is None:
            continue
        rule, details = head.groups()
        sources = re.findall(
            r"(?:^|\s)(?:source[=:]\s*)?([^\s]+):([0-9]+)(?=\s|$)", details
        )
        if len(sources) != 1:
            raise ValueError("Rule listing requires one source file and numeric line")
        source_file, source_line = sources[0]
        if re.search(r"\bkind\s*[=:]", details):
            kind = text_field(details, "kind")
        else:
            kinds = re.findall(r"(?:^|\s)(mechanical|judged)(?=\s|$)", details)
            if len(kinds) != 1:
                raise ValueError("Rule listing requires one rule kind")
            kind = kinds[0]
        item: JsonObject = {
            "rule": rule,
            "source": {"file": source_file, "line": int(source_line)},
            "kind": kind,
            "authored": text_field(details, "authored"),
            "configured": text_field(details, "configured"),
        }
        if for_file:
            origin: JsonObject = {"kind": text_field(details, "origin")}
            if origin["kind"] == "override":
                index = text_field(details, "index")
                if not re.fullmatch(r"[0-9]+", index):
                    raise ValueError("Override index is not an integer")
                origin["index"] = int(index)
            item["for_file"] = {
                "setting": text_field(details, "effective"),
                "decided_by": origin,
            }
        records.append(item)
    return records


def require_rules(
    output_text: str, case: FixtureCase, root: Path | None = None
) -> None:
    if (
        case.rules_rule is None
        or case.rules_source_file is None
        or case.rules_source_line is None
    ):
        raise ValueError("Rule-list fixture lacks literal rule/source expectations")
    try:
        output = JSON_VALUE.validate_json(output_text)
    except ValueError:
        values: list[JsonValue] = list(
            readable_rules(output_text, case.rules_file is not None)
        )
    else:
        values = (
            as_array(output["rules"]) if isinstance(output, dict) else as_array(output)
        )
    if len(values) != 1:
        raise ValueError("--rules must include exactly the loaded rule, including off")
    item = as_object(values[0])
    source = as_object(item["source"])
    if selector_name(item["rule"]) != case.rules_rule:
        raise ValueError("Rule listing names the wrong rule")
    if (
        normalized_file(source["file"], root) != case.rules_source_file
        or type(source["line"]) is not int
        or source["line"] != case.rules_source_line
    ):
        raise ValueError("Rule listing names the wrong source file or line")
    if (
        enum_name(item["kind"]) != "mechanical"
        or enum_name(item["authored"]) != "error"
        or enum_name(item["configured"]) != "off"
    ):
        raise ValueError("Rule listing loses authored/configured distinction")
    if case.rules_file:
        effective = as_object(item["for_file"])
        if enum_name(effective["setting"]) != case.rules_setting or origin_value(
            effective["decided_by"]
        ) != origin_value(case.rules_origin):
            raise ValueError("--rules file chose the wrong setting/origin")


@dataclass(frozen=True, order=True, kw_only=True)
class FindingRecord:
    rule: str
    file: str
    line: int
    column: int
    severity: str
    message: str


def json_records(report: JsonObject, root: Path) -> list[FindingRecord]:
    records: list[FindingRecord] = []
    for raw in as_array(report["findings"]):
        item = as_object(raw)
        single: JsonObject = {"findings": [item]}
        finding = report_findings(single, root)[0]
        records.append(
            FindingRecord(
                rule=finding.rule,
                file=finding.file,
                line=finding.line,
                column=finding.column,
                severity=finding.severity,
                message=str(item["message"]),
            )
        )
    return sorted(records)


def default_records(output: str, root: Path) -> list[FindingRecord]:
    records: list[FindingRecord] = []
    for line in output.splitlines():
        match = re.fullmatch(
            r"\s*(.+?)\s+(error|warn|warning)\s+(?:\[([^\]]+)\]|(\S+/\S+))(?:\s+(.*))?\s*",
            line,
        )
        if match is None:
            continue
        location, severity, bracket_rule, plain_rule, message = match.groups()
        coordinates = re.fullmatch(r"(.+?):([0-9]+):([0-9]+):?", location)
        if coordinates is None:
            raise ValueError("Default finding lacks exact line and column coordinates")
        file, line_number, column = coordinates.groups()
        records.append(
            FindingRecord(
                rule=bracket_rule or plain_rule,
                file=normalized_file(file, root),
                line=int(line_number),
                column=int(column),
                severity="warn" if severity == "warning" else severity,
                message=message or "",
            )
        )
    return sorted(records)


def github_records(output: str, root: Path) -> list[FindingRecord]:
    records: list[FindingRecord] = []
    for line in output.splitlines():
        if not re.match(r"^::(?:error|warning)(?:\s|::)", line):
            continue
        match = re.fullmatch(r"::(error|warning)\s*([^:]*)::(.*)", line)
        if match is None:
            raise ValueError("Malformed GitHub finding annotation")
        severity, properties, payload = match.groups()
        attributes: dict[str, str] = {}
        for property in properties.split(","):
            key, separator, value = property.partition("=")
            if not separator or key in attributes:
                raise ValueError("Malformed or duplicate GitHub annotation property")
            attributes[key] = unquote(value)
        if not all(
            re.fullmatch(r"[0-9]+", attributes.get(key, "")) for key in ("line", "col")
        ):
            raise ValueError("GitHub finding lacks exact numeric line and column")
        message = re.fullmatch(
            r"(?:\[([^\]]+)\]|(\S+/\S+))\s*(.*)", unquote(payload), flags=re.DOTALL
        )
        if message is None:
            raise ValueError("GitHub finding lacks its rule and message")
        bracket_rule, plain_rule, text = message.groups()
        records.append(
            FindingRecord(
                rule=bracket_rule or plain_rule,
                file=normalized_file(attributes.get("file"), root),
                line=int(attributes["line"]),
                column=int(attributes["col"]),
                severity="warn" if severity == "warning" else severity,
                message=text,
            )
        )
    return sorted(records)


def sarif_records(output: str, root: Path) -> list[FindingRecord]:
    sarif = JSON_OBJECT.validate_json(output)
    if sarif["version"] != "2.1.0":
        raise ValueError("Wrong SARIF version")
    records: list[FindingRecord] = []
    for raw_run in as_array(sarif["runs"]):
        for raw in as_array(as_object(raw_run)["results"]):
            item = as_object(raw)
            location = as_object(
                as_object(as_array(item["locations"])[0])["physicalLocation"]
            )
            region = as_object(location["region"])
            records.append(
                FindingRecord(
                    rule=str(item["ruleId"]),
                    file=normalized_file(
                        as_object(location["artifactLocation"])["uri"], root
                    ),
                    line=int(str(region["startLine"])),
                    column=int(str(region["startColumn"])),
                    severity="warn"
                    if item["level"] == "warning"
                    else str(item["level"]),
                    message=str(as_object(item["message"])["text"]),
                )
            )
    return sorted(records)


def require_format(
    output: str, report: JsonObject, format_name: str, root: Path
) -> None:
    match format_name:
        case "default":
            actual = default_records(output, root)
        case "github":
            actual = github_records(output, root)
        case "sarif":
            actual = sarif_records(output, root)
        case _:
            raise ValueError("Select a supported non-JSON output format")
    if actual != json_records(report, root):
        raise ValueError(
            f"{format_name}: finding rule/file/line/column/severity/message multiset differs from JSON"
        )


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
        judged_rules = {
            finding.rule for finding in case.findings if finding.kind == "judged"
        }
        for raw in as_array(report["findings"]):
            item = as_object(raw)
            if selector_name(item["rule"]) not in judged_rules:
                continue
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
