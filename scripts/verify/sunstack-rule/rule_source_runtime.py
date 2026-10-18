"""Own local rule-source process runs and their immutable target/evidence identity."""

import argparse
import hashlib
import json
import os
import subprocess
import sys
import typing as t
from dataclasses import dataclass
from pathlib import Path

from pydantic import BaseModel, ConfigDict, Field, JsonValue, TypeAdapter

type JsonObject = dict[str, JsonValue]
JSON_OBJECT = TypeAdapter(JsonObject)
REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
FIXTURES = Path(__file__).resolve().parent / "fixtures"


def as_object(value: JsonValue) -> JsonObject:
    assert isinstance(value, dict), "Expected a JSON object"
    return value


def as_array(value: JsonValue) -> list[JsonValue]:
    assert isinstance(value, list), "Expected a JSON array"
    return value


def object_at(value: JsonValue, *keys: str) -> JsonObject:
    for key in keys:
        value = as_object(value)[key]
    return as_object(value)


def write_json(path: Path, value: JsonObject) -> None:
    _ = path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")


def read_json(path: Path) -> JsonObject:
    return JSON_OBJECT.validate_json(path.read_bytes())


def printed_rule_definitions(outcome: JsonObject) -> dict[str, JsonObject]:
    assert outcome["kind"] == "checked", "Expected a clean check result"
    printed_rules = as_array(outcome["printed_rules"])
    checked_count = outcome["checked_rule_count"]
    assert type(checked_count) is int and checked_count == len(printed_rules), (
        "Printed rules do not cover the checked rule count"
    )
    definitions: dict[str, JsonObject] = {}
    for printed in printed_rules:
        definition = object_at(printed, "definition")
        name = definition["rule_id"]
        assert isinstance(name, str) and name not in definitions, (
            "Each printed rule needs a distinct semantic name"
        )
        definitions[name] = definition
    return definitions


class OperationResult(BaseModel):
    model_config: t.ClassVar[ConfigDict] = ConfigDict(extra="forbid", frozen=True)
    operation: t.Literal["check_rules", "evaluate_rules"]
    outcome: JsonObject


class OperationFrame(BaseModel):
    model_config: t.ClassVar[ConfigDict] = ConfigDict(extra="forbid", frozen=True)
    kind: t.Literal["operation_result"]
    protocol_version: t.Annotated[int, Field(strict=True, ge=1, le=1)]
    result: OperationResult


def product_identity() -> str:
    return subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=REPOSITORY_ROOT, text=True
    ).strip()


@dataclass(frozen=True, kw_only=True)
class JourneyTarget:
    binary: Path
    commit: str
    evidence: Path
    binary_digest: str

    def confirm(self) -> None:
        assert product_identity() == self.commit, "Declared product HEAD changed"
        product_paths = [
            "crates",
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            ".cargo",
            "scripts/cargo-debug-signing-runner.sh",
        ]
        _ = subprocess.run(
            [
                "git",
                "diff",
                "--quiet",
                self.commit,
                "--",
                *product_paths,
            ],
            cwd=REPOSITORY_ROOT,
            check=True,
        )
        status = subprocess.check_output(
            [
                "git",
                "--no-optional-locks",
                "status",
                "--porcelain",
                "--untracked-files=all",
                "--",
                *product_paths,
            ],
            cwd=REPOSITORY_ROOT,
            text=True,
        )
        assert not status, f"Product files changed: {status}"
        assert (
            hashlib.sha256(self.binary.read_bytes()).hexdigest() == self.binary_digest
        )

    def run(
        self,
        name: str,
        operation: t.Literal["check_rules", "evaluate_rules"],
        input_value: JsonObject,
        *,
        expected_exit: int,
    ) -> JsonObject:
        self.confirm()
        directory = self.evidence / name
        directory.mkdir(exist_ok=False)
        write_json(
            directory / "stdin.json",
            {"protocol_version": 1, "operation": operation, "input": input_value},
        )
        # These journeys never inherit a provider credential or proxy. The runner
        # only needs the local tool path and user/runtime directories for signing.
        environment = {
            key: os.environ[key]
            for key in ["PATH", "HOME", "TMPDIR", "LANG", "SUNSTACK_DEBUG_SIGNING_TEAM"]
            if key in os.environ
        }
        command = [
            str(REPOSITORY_ROOT / "scripts/cargo-debug-signing-runner.sh"),
            str(self.binary),
        ]
        with (
            (directory / "stdin.json").open("rb") as stdin,
            (directory / "stdout.json").open("wb") as stdout,
            (directory / "stderr.log").open("wb") as stderr,
        ):
            process = subprocess.run(
                command,
                cwd=REPOSITORY_ROOT,
                env=environment,
                stdin=stdin,
                stdout=stdout,
                stderr=stderr,
                timeout=30,
                check=False,
            )
        write_json(
            directory / "result.json",
            {
                "command": [argument for argument in command],
                "cwd": str(REPOSITORY_ROOT),
                "commit": self.commit,
                "binary_sha256": self.binary_digest,
                "exit_code": process.returncode,
                "provider_credential_present": False,
            },
        )
        self.confirm()
        assert process.returncode == expected_exit, f"Unexpected exit: {directory}"
        stderr_text = (directory / "stderr.log").read_text()
        assert "sunstack-rule: ready\n" in stderr_text, f"No readiness: {directory}"
        frame = OperationFrame.model_validate_json(
            (directory / "stdout.json").read_bytes()
        )
        assert frame.result.operation == operation
        return frame.result.outcome


class JourneyArguments(BaseModel):
    model_config: t.ClassVar[ConfigDict] = ConfigDict(extra="forbid", frozen=True)
    binary: Path
    evidence: Path
    commit: str


def journey_target(description: str) -> JourneyTarget:
    if sys.flags.optimize:
        raise SystemExit("Unset PYTHONOPTIMIZE and omit -O/-OO; assertions are proof")
    parser = argparse.ArgumentParser(description=description)
    _ = parser.add_argument(
        "--binary", type=Path, required=True, help="Declared built sunstack-rule"
    )
    _ = parser.add_argument(
        "--commit", required=True, help="Full declared product commit at HEAD"
    )
    _ = parser.add_argument(
        "--evidence", type=Path, required=True, help="New owned evidence directory"
    )
    arguments = JourneyArguments.model_validate(vars(parser.parse_args()))
    assert product_identity() == arguments.commit, "Declared product commit is not HEAD"
    binary = Path(arguments.binary).resolve(strict=True)
    evidence = Path(arguments.evidence).resolve()
    evidence.mkdir(parents=True, exist_ok=False)
    # Sign before freezing the binary hash: the supported runner may atomically
    # replace an unsigned executable on its first use.
    with (evidence / "signing-preflight.log").open("wb") as output:
        _ = subprocess.run(
            [
                str(REPOSITORY_ROOT / "scripts/cargo-debug-signing-runner.sh"),
                "--sign-only",
                str(binary),
            ],
            cwd=REPOSITORY_ROOT,
            stdout=output,
            stderr=subprocess.STDOUT,
            check=True,
            timeout=30,
        )
    target = JourneyTarget(
        binary=binary,
        commit=arguments.commit,
        evidence=evidence,
        binary_digest=hashlib.sha256(binary.read_bytes()).hexdigest(),
    )
    target.confirm()
    return target


def rule_entry(name: str) -> JsonObject:
    return {
        "name": name,
        "target": {
            "input": "subject",
            "select": {"kind": "paragraph", "target_id": "U001"},
        },
        "threshold": 0.8,
    }


def source_fixture(root: Path, markdown: str, rules: JsonObject) -> JsonObject:
    root.mkdir(exist_ok=False)
    _ = (root / "source.md").write_text(markdown)
    write_json(root / "source.rules.json", rules)
    write_json(
        root / "set.json",
        {
            "schema_version": 1,
            "name": "markdown-source-journey",
            "description": "Local rule source verification",
            "inputs": {"subject": {"kind": "document"}},
            "applicability": [],
            "common_ruleset_instructions": "Judge only the named target.",
            "marker_prefix": "K",
            "rule_sources": [{"markdown": "source.md", "rules": "source.rules.json"}],
        },
    )
    return {
        "definition_root": str(root),
        "definitions": {
            "kind": "rule_sets",
            "rule_sets": [
                {
                    "path": "set.json",
                    "binding_groups": [
                        {
                            "bindings": {
                                "subject": {
                                    "kind": "document",
                                    "repository": "repo",
                                    "path": "reviewed.md",
                                }
                            }
                        }
                    ],
                }
            ],
        },
        "repositories": [
            {
                "key": "repo",
                "root": str(root),
                "input_paths": [{"kind": "glob", "pattern": "**"}],
                "change_scope": {"kind": "full"},
            }
        ],
        "payload_schemas": [],
        "limits": {
            "max_files": 32,
            "max_file_bytes": 262144,
            "max_total_bytes": 1048576,
            "max_matches": 100,
            "max_transcript_bytes": 262144,
            "include_ignored": False,
        },
        "output": "assembled_rules",
    }
