# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic==2.14.1", "pydantic_core==2.50.1", "jsonschema==4.25.1"]
# ///
"""Compare shipped config structure with binary checks, keeping semantics apart."""

import argparse
import copy
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

from jsonschema import Draft202012Validator
from lint_observations import require_diagnostic
from runner_containers import ContainerRuntime
from runner_contracts import (
    JSON_OBJECT,
    FixtureCase,
    FixtureManifest,
    JsonObject,
    JsonValue,
    write_json,
)
from runner_fixture_setup import base_config, mechanical_rule, write_fixture
from runner_process import RunnerTarget, run_case
from scenario_transport import ControlledServer
from schema_fixtures import (
    authored_location,
    structural_config_negatives,
    weakened_schema_sensitivity,
)

SEMANTIC_REFUSALS = {
    "bad-glob",  # Specification R15/V12 at 59b12d3: glob grammar is binary-only.
    "limit-integral-decimal",  # R15 at 26122bf: numeric spelling is binary-only.
    "missing-ruleset",
    "same-ruleset",
    "unknown-selector",
    "unsupported-input",
    "old-block",
    "old-nudge",
    "old-band-field",
}
STRUCTURAL_REFUSALS = {
    "unknown-field",
    "limit-zero",
    "limit-fractional",
    "limit-u32-overflow",
    "limit-u64-overflow",
}
PARSER_REFUSALS = {"syntax", "duplicate-key", "duplicate-entry"}


def config_check_expectation(name: str) -> tuple[str, bool | None, int]:
    if name in PARSER_REFUSALS:
        return "parser", None, 2
    if name in SEMANTIC_REFUSALS:
        return "semantic", True, 2
    if name in STRUCTURAL_REFUSALS:
        return "structural", False, 2
    return "structural", True, 0


def unique_object(pairs: list[tuple[str, JsonValue]]) -> JsonObject:
    result: JsonObject = {}
    for name, value in pairs:
        if name in result:
            raise ValueError("Duplicate JSON name is a parser refusal")
        result[name] = value
    return result


def config_instance(text: str) -> JsonObject:
    return JSON_OBJECT.validate_python(
        json.loads(text, object_pairs_hook=unique_object)
    )


def reject_external_references(value: JsonValue) -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            if key in ("$ref", "$dynamicRef") and (
                not isinstance(child, str) or not child.startswith("#")
            ):
                raise ValueError("Schema observation permits only local references")
            reject_external_references(child)
    elif isinstance(value, list):
        for child in value:
            reject_external_references(child)


def compare_acceptance(
    category: str,
    expected_schema: bool | None,
    expected_binary: int,
    schema_accepts: bool | None,
    binary_exit: int,
) -> bool:
    if category == "parser":
        return schema_accepts is None and binary_exit == expected_binary
    return schema_accepts == expected_schema and binary_exit == expected_binary


def observe_schema(
    target: RunnerTarget,
    schema_path: Path,
    fixtures: Path,
    manifest: FixtureManifest,
    evidence: Path,
    server: ControlledServer,
) -> None:
    evidence.mkdir(parents=True, exist_ok=False)
    schema_bytes = schema_path.read_bytes()
    schema = JSON_OBJECT.validate_json(schema_bytes)
    reject_external_references(schema)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema)
    (evidence / "released-schema.json").write_bytes(schema_bytes)
    rows: list[JsonObject] = []
    cases: list[tuple[FixtureCase, Path, str, bool | None, int]] = []
    for original in manifest.cases:
        if original.name == "missing-config":
            rows.append(
                {
                    "case": original.name,
                    "category": "filesystem",
                    "schema_accepts": None,
                    "reason": "No config file exists; schema has no input instance",
                    "matched": True,
                }
            )
            continue
        category, expected_schema, expected_exit = config_check_expectation(
            original.name
        )
        cases.append(
            (
                original,
                fixtures / original.name,
                category,
                expected_schema,
                expected_exit,
            )
        )
    canonical = evidence / "canonical-c4"
    config: JsonObject = {
        "$schema": "https://github.com/ShravanSunder/sunstack-tools/releases/download/v0.1.0/sunstack.lint.schema.json",
        "rule_sets": {
            "skills": {
                "path": "rules/skill-standards.ruleset.json",
                "files": ["plugins/**/SKILL.md"],
            }
        },
        "rules": {"skills/*": "error", "skills/skill.load-result": "warn"},
        "overrides": [
            {"files": ["plugins/**/retired-skills/**"], "rules": {"skills/*": "off"}}
        ],
        "ignore_patterns": ["tmp/**"],
        "backend": {"kind": "jev_openrouter", "model": "jev-latest"},
    }
    rule = mechanical_rule()
    rule["rule_id"] = "skill.load-result"
    write_fixture(canonical, config=config, rules=[rule])
    (canonical / "rules/skill-standards.ruleset.json").write_bytes(
        (canonical / "rules/set.json").read_bytes()
    )
    cases.append(
        (
            FixtureCase(name="canonical-c4", wrong_build="C4 config is refused"),
            canonical,
            "structural",
            True,
            0,
        )
    )
    for name, field, value in (
        ("limit-zero", "max_files", 0),
        ("limit-fractional", "max_files", 1.5),
        ("limit-integral-decimal", "max_files", 1.0),
        ("limit-u32-overflow", "max_files", 2**32),
        ("limit-u64-overflow", "max_total_bytes", 2**64),
    ):
        fixture = evidence / name
        limit_config = copy.deepcopy(base_config())
        limit_config["limits"] = {field: value}
        write_fixture(fixture, config=limit_config)
        authored = (fixture / "sunstack.lint.json").read_text().splitlines()
        line_index, element = next(
            (index, text) for index, text in enumerate(authored) if f'"{field}"' in text
        )
        original = FixtureCase(
            name=name,
            diagnostic_file="sunstack.lint.json",
            diagnostic_line=line_index + 1,
            diagnostic_column=element.index(f'"{field}"') + 1,
            diagnostic_column_end=len(element),
            wrong_build="Config limit disagrees with its native nonzero integer type or written form",
        )
        cases.append(
            (
                original,
                fixture,
                *config_check_expectation(name),
            )
        )
    structural_cases = structural_config_negatives()
    sensitivity = weakened_schema_sensitivity(schema, structural_cases)
    write_json(
        evidence / "sensitivity.json",
        {
            "changed_verdicts": sensitivity,
            "all_axes_sensitive": all(sensitivity.values()),
        },
    )
    for negative in structural_cases:
        fixture = evidence / negative.name
        write_fixture(fixture, config=negative.config)
        line, first, last = authored_location(
            negative, (fixture / "sunstack.lint.json").read_text()
        )
        original = FixtureCase(
            name=negative.name,
            diagnostic_file="sunstack.lint.json",
            diagnostic_line=line,
            diagnostic_column=first,
            diagnostic_column_end=last,
            wrong_build="Config structure is accepted or loses its authored location",
        )
        cases.append((original, fixture, "structural", False, 2))
    for original, fixture, category, expected_schema, expected_exit in cases:
        config_file = fixture / original.selected_config
        raw = config_file.read_text()
        errors: list[JsonValue] = []
        try:
            instance = config_instance(raw)
        except ValueError:
            accepts = None
        else:
            errors = [error.message for error in validator.iter_errors(instance)]
            accepts = not errors
        relative_config = os.path.relpath(config_file, fixture / original.cwd)
        check = original.model_copy(
            update={
                "arguments": ["lint", "--check", "--config", relative_config],
                "mode": "check",
                "exit_code": expected_exit,
                "requests": 0,
                "cancel": False,
                "credential": False,
                "unreadable": None,
            }
        )
        directory = evidence / "processes" / original.name
        process_error = None
        try:
            exit_code, _, _ = run_case(target, fixture, directory, check, server)
        except ValueError as error:
            if not str(error).startswith(f"{check.name}: exit "):
                raise
            process_error = str(error)
            exit_code = JSON_OBJECT.validate_json(
                (directory / "process.json").read_bytes()
            )["exit_code"]
            if type(exit_code) is not int:
                raise ValueError(
                    "Schema observation lost its actual process exit"
                ) from None
        if expected_exit == 2 and exit_code == 2 and check.diagnostic_file:
            try:
                require_diagnostic(
                    "",
                    check,
                    (directory / "stderr.log").read_text(),
                    target.comparison_root(fixture),
                )
            except ValueError as error:
                process_error = str(error)
        matched = (
            compare_acceptance(
                category, expected_schema, expected_exit, accepts, exit_code
            )
            and process_error is None
        )
        rows.append(
            {
                "case": original.name,
                "category": category,
                "config_sha256": hashlib.sha256(raw.encode()).hexdigest(),
                "expected_schema_accepts": expected_schema,
                "schema_accepts": accepts,
                "expected_binary_exit": expected_exit,
                "binary_exit": exit_code,
                "schema_errors": errors,
                "process_error": process_error,
                "matched": matched,
                "evidence": str(directory),
            }
        )
        write_json(
            evidence / "comparison.json",
            {"schema_sha256": hashlib.sha256(schema_bytes).hexdigest(), "rows": rows},
        )
    if schema_path.read_bytes() != schema_bytes:
        raise ValueError("Released schema changed during observation")
    mismatches = [row["case"] for row in rows if not row["matched"]]
    write_json(
        evidence / "result.json",
        {
            "schema_sha256": hashlib.sha256(schema_bytes).hexdigest(),
            "cases": len(rows),
            "mismatches": mismatches,
            "parser_note": "Duplicate names/syntax are parser refusals, not JSON Schema assertions",
            "sensitivity": sensitivity,
        },
    )
    if mismatches or not all(sensitivity.values()):
        raise ValueError(
            f"Released-schema agreement failed: {mismatches}; insensitive axes: {[name for name, changed in sensitivity.items() if not changed]}; inspect {evidence}"
        )


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Compare an exact released schema with binary --check on completed S1 fixtures; no model calls. Parser-only and filesystem refusals are kept separate from schema structure."
    )
    parser.add_argument("--schema", required=True, type=Path)
    parser.add_argument(
        "--fixtures",
        required=True,
        type=Path,
        help="Completed S1 fixtures directory with manifest.json",
    )
    parser.add_argument("--evidence", required=True, type=Path)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--source-root", required=True, type=Path)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--container-image")
    parser.add_argument("--container-control-host")
    args = parser.parse_args()
    if sys.flags.optimize:
        parser.error("Unset PYTHONOPTIMIZE and omit -O/-OO")
    if bool(args.container_image) != bool(args.container_control_host):
        parser.error("Container mode needs both image and declared control hostname")
    try:
        from_digest = hashlib.sha256(args.binary.read_bytes()).hexdigest()
        runtime = (
            ContainerRuntime(
                image=args.container_image, control_host=args.container_control_host
            )
            if args.container_image
            else None
        )
        target = RunnerTarget(
            binary=args.binary.resolve(),
            source=args.source_root.resolve(),
            commit=args.commit,
            digest=from_digest,
            container=runtime,
        )
        target.confirm()
        if runtime:
            runtime.confirm_image()
        evidence = args.evidence.resolve()
        evidence.mkdir(parents=True, exist_ok=False)
        manifest = FixtureManifest.model_validate_json(
            (args.fixtures / "manifest.json").read_bytes()
        )
        with ControlledServer(evidence / "http") as server:
            observe_schema(
                target,
                args.schema.resolve(),
                args.fixtures.resolve(),
                manifest,
                evidence / "comparison",
                server,
            )
        target.confirm()
        print(
            "Released-schema fixture comparisons passed; parser/filesystem distinctions retained"
        )
    except (
        ValueError,
        TypeError,
        OSError,
        TimeoutError,
        subprocess.SubprocessError,
    ) as error:
        parser.exit(2, f"Schema setup/observation failed: {error}\n")


if __name__ == "__main__":
    main()
