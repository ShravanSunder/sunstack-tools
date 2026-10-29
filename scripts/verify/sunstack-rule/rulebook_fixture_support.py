"""Prepare the adopted S0 fixtures and independent rulebook journey expectations."""

import json
import os
import shutil
import subprocess
import typing as t
from pathlib import Path

from pydantic import JsonValue, TypeAdapter

type JsonObject = dict[str, JsonValue]
JSON_VALUE = TypeAdapter(JsonValue)
JSON_OBJECT = TypeAdapter(JsonObject)
REPOSITORY = Path(__file__).resolve().parents[3]
FIXTURE = REPOSITORY / "crates/sunstack/tests/fixtures/rulebook-lint"
CONFIG = "skill-rules.config.json"
RULES = "rulebook/steering.rules.json"
RULEBOOK = "rulebook/steering.md"
SKILL = "skills/verification-design/SKILL.md"
ORACLE = (
    "docs/specs/2026-10-29-skill-rules-evidence-and-decisions/verification-design.md"
)


def read_object(path: Path) -> JsonObject:
    return JSON_OBJECT.validate_json(path.read_bytes())


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(JSON_VALUE.validate_python(value), indent=2) + "\n")


def object_value(value: JsonValue) -> JsonObject:
    if not isinstance(value, dict):
        raise TypeError("Expected JSON object")
    return value


def array_value(value: JsonValue) -> list[JsonValue]:
    if not isinstance(value, list):
        raise TypeError("Expected JSON array")
    return value


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def prepare_fixture(destination: Path, *, markers: tuple[str, ...] = ()) -> Path:
    shutil.copytree(FIXTURE, destination)
    if markers:
        rules = read_object(destination / RULES)
        write_json(destination / RULES, {name: rules[name] for name in markers})
        lines = (destination / RULEBOOK).read_text().splitlines()
        (destination / RULEBOOK).write_text(
            "\n".join(
                line
                for line in lines
                if any(f"<!-- {name} -->" in line for name in markers)
            )
            + "\n"
        )
    return destination


def configure(root: Path, change: t.Callable[[JsonObject], None]) -> None:
    config = read_object(root / CONFIG)
    change(config)
    write_json(root / CONFIG, config)


def select_worked_skill(root: Path) -> None:
    configure(root, lambda config: config.update(check={"files": [SKILL]}))


def git_command(root: Path, *arguments: str) -> str:
    environment = {
        "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
        "HOME": str(root.parent / "git-home"),
        "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_CONFIG_GLOBAL": os.devnull,
        "GIT_AUTHOR_NAME": "Rulebook fixture",
        "GIT_AUTHOR_EMAIL": "fixture@example.invalid",
        "GIT_COMMITTER_NAME": "Rulebook fixture",
        "GIT_COMMITTER_EMAIL": "fixture@example.invalid",
        "GIT_AUTHOR_DATE": "2026-10-29T00:00:00Z",
        "GIT_COMMITTER_DATE": "2026-10-29T00:00:00Z",
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
        text=True,
        capture_output=True,
        check=True,
        timeout=15,
    ).stdout.strip()


def baseline_git(root: Path) -> None:
    git_command(root, "init", "-q")
    git_command(root, "add", ".")
    git_command(root, "commit", "-qm", "Adopted rulebook fixture")


def oracle_matches(root: Path) -> list[JsonObject]:
    matches = array_value(read_object(root / "expected/scan-oracle.json")["matches"])
    return [object_value(item) for item in matches]


def expected_steps(root: Path) -> list[JsonObject]:
    return [
        item
        for item in oracle_matches(root)
        if item["rule"] == "step" and item["file"] == SKILL
    ]
