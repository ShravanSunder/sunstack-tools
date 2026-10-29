# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic==2.14.1", "pydantic_core==2.50.1"]
# ///
"""S1 current rulebook lint: retained exits, formats, no-key gate, init and scopes.

Run from the monorepo root with --prepare --evidence <new-dir>; no product runs.
Omit --prepare and add --binary, --source-root, full --commit to observe a local
declared target. --case limits a rerun and does not earn a full-route result.
Git picking and rule/target change coverage join rulebook_lint.py; native schema,
library/CLI equality and exec proof remain mandatory current-target joins.
This driver uses only owned loopback HTTP and synthetic authorization.
"""

import argparse
import re
import shutil
import subprocess
import sys
from pathlib import Path

from pydantic import BaseModel, ConfigDict, Field
from rulebook_fixture_support import (
    CONFIG,
    FIXTURE,
    ORACLE,
    RULES,
    SKILL,
    JsonObject,
    array_value,
    configure,
    object_value,
    prepare_fixture,
    read_object,
    require,
    select_worked_skill,
    write_json,
)
from rulebook_lint import finding_line, findings, lint_args, status
from rulebook_process import RulebookProcess, declared_target, response_body


class LintCase(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True)
    name: str
    exit: int
    status: str | None = None
    arguments: list[str] = Field(default_factory=lambda: lint_args("--all"))
    severity: str | None = None
    marker: str = "K002"
    requests: int = 0
    credential: bool = False
    cancel: bool = False
    format: str = "json"


def prepare(destination: Path) -> list[LintCase]:
    destination.mkdir(parents=True, exist_ok=False)
    cases = [
        LintCase(name="mechanical-error", exit=1, status="findings", severity="error"),
        LintCase(name="clean", exit=0, status="clean"),
        LintCase(name="warning", exit=0, status="clean", severity="warn"),
        LintCase(
            name="warning-limit",
            exit=1,
            status="findings",
            severity="warn",
            arguments=lint_args("--all", "--max-warnings", "0"),
        ),
        LintCase(name="ignored", exit=0, status="nothing_selected"),
        LintCase(name="empty-candidates", exit=0, status="nothing_selected"),
        LintCase(name="missing-key", exit=2, status="invalid", marker="K003"),
        LintCase(
            name="judged",
            exit=1,
            status="findings",
            severity="error",
            marker="K003",
            requests=1,
            credential=True,
        ),
        LintCase(
            name="cancelled",
            exit=4,
            status="cancelled",
            marker="K003",
            requests=1,
            credential=True,
            cancel=True,
        ),
        LintCase(name="file-limit", exit=3, status="incomplete"),
        LintCase(
            name="check", exit=0, arguments=["lint", "--config", CONFIG, "--check"]
        ),
        LintCase(name="invalid", exit=2, status="invalid"),
        LintCase(
            name="named-path",
            exit=1,
            status="findings",
            severity="error",
            arguments=lint_args(SKILL),
        ),
        LintCase(
            name="default-discovery",
            exit=1,
            status="findings",
            severity="error",
            arguments=["lint", "--all", "--format", "json"],
        ),
        LintCase(name="init", exit=0, arguments=["lint", "--init"]),
    ]
    cases.extend(
        LintCase(
            name=f"format-{format_name}",
            exit=1,
            severity="error",
            format=format_name,
            arguments=["lint", "--config", CONFIG, "--all", "--format", format_name],
        )
        for format_name in ("default", "github", "sarif")
    )
    for case in cases:
        if case.name == "init":
            (destination / case.name).mkdir()
            continue
        root = prepare_fixture(destination / case.name, markers=(case.marker,))
        select_worked_skill(root)
        rules = read_object(root / RULES)
        entry = object_value(rules[case.marker])
        if case.name in ("warning", "warning-limit"):
            entry["severity"] = "warn"
        if case.marker == "K003":
            entry["severity"] = "error"
        write_json(root / RULES, rules)
        if case.name == "clean":
            path = root / SKILL
            path.write_text(
                re.sub(
                    r"(?m)^description:.*$",
                    'description: "Use for a short description."',
                    path.read_text(),
                )
            )
        if case.name == "ignored":
            configure(
                root,
                lambda config: object_value(config["check"]).update(ignore=[SKILL]),
            )
        if case.name == "empty-candidates":
            configure(
                root,
                lambda config: object_value(config["check"]).update(
                    files=["absent/*.md"]
                ),
            )
        if case.name == "file-limit":
            configure(root, lambda config: config.update(limits={"max_file_bytes": 1}))
        if case.name == "invalid":
            configure(root, lambda config: config.update(unknown=True))
        if case.name == "default-discovery":
            (root / "sunstack.lint.json").write_bytes((root / CONFIG).read_bytes())
        write_json(root / "expected-results.json", case.model_dump(mode="json"))
    write_json(
        destination / "manifest.json", [case.model_dump(mode="json") for case in cases]
    )
    return cases


def require_report(report: JsonObject, case: LintCase) -> None:
    require(status(report) == case.status, f"{case.name}: wrong status")
    rows = findings(report)
    counts = object_value(report["counts"])
    for severity in ("error", "warn"):
        require(
            counts[severity] == sum(item["severity"] == severity for item in rows),
            f"{case.name}: severity counts do not match findings",
        )
    if case.severity:
        require(len(rows) == 1, f"{case.name}: extra or missing findings")
        item = rows[0]
        require(
            item["marker"] == case.marker
            and item["file"] == SKILL
            and finding_line(item) == 3
            and item["severity"] == case.severity,
            f"{case.name}: wrong finding identity/location/severity",
        )
        require(bool(item["message"]), "Finding has no user-visible message")
    else:
        require(not rows, f"{case.name}: unexpected findings")
    usage = object_value(report["usage"])
    require(
        usage["model_requests"] == case.requests,
        f"{case.name}: reported requests differ",
    )
    if case.name in ("missing-key", "invalid"):
        require(
            usage["checked_files_acquired"] == 0,
            "Invalid/no-key run acquired checked content",
        )
    if case.name == "judged":
        require(
            bool(object_value(report["evidence_seen"])),
            "Judged report lost sent evidence",
        )
        answers = object_value(object_value(rows[0]["judgment"])["answers"])
        answer = object_value(answers["names_trigger"])
        require(
            answer["type"] == "noul" and answer["yes_probability"] == 0.1,
            "Judged finding lost the native supplied answer",
        )
    if case.name in ("cancelled", "file-limit"):
        require(bool(array_value(report["incomplete"])), "Unresolved work missing")


def observe_init(process: RulebookProcess, root: Path) -> None:
    process.run(root, "init-create", ["lint", "--init"], expected_exit=0)
    path = root / "sunstack.lint.json"
    skeleton = read_object(path)
    require(
        "rulebook" in skeleton and "rule_sets" not in skeleton,
        "Init produced old config",
    )
    require(
        {item.name for item in root.iterdir()} == {"sunstack.lint.json"},
        "Init wrote unexpected files",
    )
    original = path.read_bytes()
    process.run(root, "init-no-overwrite", ["lint", "--init"], expected_exit=2)
    require(path.read_bytes() == original, "Init overwrote existing bytes")
    process.run(root, "init-empty-check", ["lint", "--check"], expected_exit=2)
    shutil.copytree(FIXTURE, root, dirs_exist_ok=True)
    path.write_bytes((root / CONFIG).read_bytes())
    process.run(root, "init-populated-check", ["lint", "--check"], expected_exit=0)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--prepare", action="store_true")
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--source-root", type=Path, default=Path.cwd())
    parser.add_argument("--commit")
    parser.add_argument("--case", action="append", default=[])
    args = parser.parse_args()
    if sys.flags.optimize:
        parser.error("Unset PYTHONOPTIMIZE and omit -O/-OO")
    if not args.prepare and (args.binary is None or args.commit is None):
        parser.error("Observation requires --binary and full --commit")
    try:
        target = (
            None
            if args.prepare
            else declared_target(args.binary, args.source_root, args.commit)
        )
        evidence = args.evidence.resolve()
        evidence.mkdir(parents=True, exist_ok=False)
        cases = prepare(evidence / "fixtures")
        selected = [case for case in cases if not args.case or case.name in args.case]
        require(
            bool(selected) and set(args.case).issubset({case.name for case in cases}),
            "Unknown or empty case selection",
        )
        if args.prepare:
            write_json(
                evidence / "prepared.json",
                {"product_observed": False, "cases": len(cases), "oracle": ORACLE},
            )
            print(f"Prepared {len(cases)} current S1 cases; product unobserved")
            return
        if target is None:
            raise ValueError("Declared target missing")
        with RulebookProcess(target=target, evidence=evidence / "run") as process:
            process.reply = lambda body: response_body(body, 0.1, "outcome")
            for case in selected:
                root = evidence / "fixtures" / case.name
                if case.name == "init":
                    observe_init(process, root)
                    continue
                configure(
                    root,
                    lambda config: object_value(config["decision_model"]).update(
                        endpoint=process.server.endpoint
                    ),
                )
                if case.name == "default-discovery":
                    (root / "sunstack.lint.json").write_bytes(
                        (root / CONFIG).read_bytes()
                    )
                out, err, calls = process.run(
                    root,
                    case.name,
                    case.arguments,
                    expected_exit=case.exit,
                    credential=case.credential,
                    cancel=case.cancel,
                )
                require(
                    len(calls) == case.requests,
                    f"{case.name}: actual HTTP calls differ",
                )
                if case.status is not None:
                    require_report(
                        read_object(process.evidence / case.name / "stdout.log"), case
                    )
                elif case.name == "check":
                    require("K002" in (out + err).decode(), "Check lost rule listing")
                elif case.format == "sarif":
                    report = read_object(process.evidence / case.name / "stdout.log")
                    results = array_value(
                        object_value(array_value(report["runs"])[0])["results"]
                    )
                    require(len(results) == 1, "SARIF lost or duplicated finding")
                else:
                    text = out.decode()
                    require(
                        "Over 300 characters." in text
                        and (SKILL in text or "SKILL.md" in text),
                        "Rendered format lost finding/message",
                    )
        write_json(
            evidence / "s1-pass.json",
            {
                "commit": target.commit,
                "binary_sha256": target.digest,
                "executed": [case.name for case in selected],
                "full_inventory": len(cases),
                "required_joins": [
                    "rulebook_lint Git/change/skip journey",
                    "native report schema and library/CLI equality",
                    "native exec JSONL/conversation tests",
                    "recording-source gate tests",
                    "Linux/container and released schema separate targets",
                ],
            },
        )
        print(f"Observed {len(selected)} S1 cases; remaining target/native joins named")
    except (
        ValueError,
        TypeError,
        KeyError,
        OSError,
        subprocess.SubprocessError,
    ) as error:
        parser.exit(2, f"S1 rulebook observation failed: {error}\n")


if __name__ == "__main__":
    main()
