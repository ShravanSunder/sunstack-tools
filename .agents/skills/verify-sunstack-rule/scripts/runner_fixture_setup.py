"""Build small real repositories and literal S1 expectations; never consult product output."""

import json
import os
import subprocess
from pathlib import Path

from runner_contracts import (
    FixtureFinding,
    JsonObject,
    as_object,
    write_json,
)

BAD_TEXT = "You may skip this standard.\n"
GOOD_TEXT = "You MUST follow this standard.\n"


def mechanical_rule(severity: str | None = "error") -> JsonObject:
    value: JsonObject = {
        "rule_id": "standard",
        "target": "target",
        "applicability": [],
        "questions": {},
        "evidence": [
            {
                "id": "target",
                "input": "subject",
                "select": {
                    "kind": "markdown_units",
                    "mode": "unit",
                    "units": ["paragraph"],
                },
            }
        ],
        "verdict": {
            "kind": "mechanical",
            "explanation": "Use MUST in each paragraph.",
            "check": {"kind": "expect", "expect": {"matches": "MUST"}},
        },
    }
    if severity is not None:
        value["severity"] = severity
    return value


def judged_rule(severity: str = "error", band: float | None = None) -> JsonObject:
    value = mechanical_rule(severity)
    value["evidence"] = [
        {
            "id": "target",
            "input": "subject",
            "select": {"kind": "paragraph", "target_id": "U001"},
        }
    ]
    value["verdict"] = {
        "kind": "noul",
        "question": {
            "type": "noul",
            "instructions": "Is the statement in the supplied paragraph true?",
        },
        "threshold": 0.8,
    }
    if band is not None:
        as_object(value["verdict"])["error_at_or_below"] = band
    return value


def base_config() -> JsonObject:
    return {
        "$schema": "https://github.com/ShravanSunder/sunstack-tools/releases/download/v0.1.0/sunstack.lint.schema.json",
        "rule_sets": {"docs": {"path": "rules/set.json", "files": ["docs/*.md"]}},
        "backend": {
            "kind": "jev_openrouter",
            "model": "jev-latest",
            "endpoint": "http://127.0.0.1:9/systemone",
        },
    }


def git_command(root: Path, *arguments: str) -> str:
    environment = {
        "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
        "HOME": str(root.parent / "git-home"),
        "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_CONFIG_GLOBAL": os.devnull,
        "GIT_AUTHOR_NAME": "Fixture Author",
        "GIT_AUTHOR_EMAIL": "fixture@example.invalid",
        "GIT_COMMITTER_NAME": "Fixture Author",
        "GIT_COMMITTER_EMAIL": "fixture@example.invalid",
        "GIT_AUTHOR_DATE": "2026-10-18T00:00:00Z",
        "GIT_COMMITTER_DATE": "2026-10-18T00:00:00Z",
    }
    return subprocess.run(
        [
            "git",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgsign=false",
            *arguments,
        ],
        cwd=root,
        env=environment,
        capture_output=True,
        text=True,
        check=True,
        timeout=15,
    ).stdout.strip()


def write_fixture(
    root: Path,
    *,
    config: JsonObject | None = None,
    rules: list[JsonObject] | None = None,
    git: bool = False,
) -> None:
    root.mkdir(parents=True, exist_ok=False)
    (root / "docs").mkdir()
    (root / "rules").mkdir()
    (root / "docs/one.md").write_text(BAD_TEXT)
    write_json(
        root / "sunstack.lint.json", config if config is not None else base_config()
    )
    write_json(
        root / "rules/set.json",
        {
            "schema_version": 1,
            "name": "Runner fixture",
            "description": "Literal S1 oracle",
            "inputs": {"subject": {"kind": "document"}},
            "applicability": [],
            "common_ruleset_instructions": "Read only the supplied synthetic text.",
            "rule_paths": ["rules.jsonl"],
        },
    )
    (root / "rules/rules.jsonl").write_text(
        "\n".join(
            json.dumps(rule)
            for rule in (rules if rules is not None else [mechanical_rule()])
        )
        + "\n"
    )
    if git:
        git_command(root, "init", "-q")
        git_command(root, "add", ".")
        git_command(root, "commit", "-qm", "Fixture baseline")


def expected_finding(
    severity: str = "error",
    rule: str = "standard",
    file: str = "docs/one.md",
    kind: str = "mechanical",
) -> FixtureFinding:
    return FixtureFinding.model_validate(
        {"rule": f"docs/{rule}", "file": file, "severity": severity, "kind": kind}
    )
