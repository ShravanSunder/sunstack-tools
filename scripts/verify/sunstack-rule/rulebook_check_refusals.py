# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic==2.14.1", "pydantic_core==2.50.1"]
# ///
"""U9: one otherwise-valid fixture per R24 refusal through the real lint --check.

Run from the monorepo root with uv run --script <this-file> --prepare --evidence
<new-directory> to inspect the independently authored cases without a binary.
For observation omit --prepare and supply --binary, --source-root and full
--commit. No genuine provider request or credential access. Recording-source
and backend-construction proof is joined from rulebook_lint_gates.rs, not
inferred from an exit or from this binary's own usage counter.
"""

import argparse
import re
import subprocess
import sys
import typing as t
from pathlib import Path

from pydantic import BaseModel, ConfigDict, JsonValue
from rulebook_fixture_support import (
    CONFIG,
    ORACLE,
    RULEBOOK,
    RULES,
    JsonObject,
    configure,
    object_value,
    prepare_fixture,
    read_object,
    require,
    write_json,
)
from rulebook_process import RulebookProcess, declared_target


class RefusalCase(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True)
    name: str
    file: str
    offending: str
    reason: str


def set_field(document: JsonObject, path: tuple[str, ...], value: JsonValue) -> None:
    owner = document
    for part in path[:-1]:
        owner = object_value(owner[part])
    owner[path[-1]] = value


def prepare_cases(destination: Path) -> list[RefusalCase]:
    destination.mkdir(parents=True, exist_ok=False)
    cases: list[RefusalCase] = []

    def keep(
        name: str,
        file: str,
        offending: str,
        reason: str,
        change: t.Callable[[Path], object],
    ) -> None:
        root = prepare_fixture(destination / name)
        configure(
            root,
            lambda config: object_value(config["decision_model"]).update(
                endpoint="http://127.0.0.1:9/systemone"
            ),
        )
        change(root)
        cases.append(
            RefusalCase(name=name, file=file, offending=offending, reason=reason)
        )

    def json_change(
        name: str,
        file: str,
        path: tuple[str, ...],
        value: JsonValue,
        *,
        offending: str | None = None,
    ) -> None:
        def mutate(root: Path) -> None:
            document = read_object(root / file)
            set_field(document, path, value)
            write_json(root / file, document)

        keep(name, file, offending or path[-1], name, mutate)

    for key, value in [
        ("unknown", True),
        ("rulebook", []),
        ("marker_prefix", "kk"),
        ("max_question_words", 0),
        ("check", {"files": []}),
    ]:
        json_change(f"config-{key}", CONFIG, (key,), value)
    for key in ("rulebook", "marker_prefix", "check", "evidence", "decision_model"):

        def omit(root: Path, key: str = key) -> None:
            config = read_object(root / CONFIG)
            config.pop(key)
            write_json(root / CONFIG, config)

        keep(f"missing-{key}", CONFIG, key, "required key missing", omit)
    for key, replacement in [
        ("rule_sets", "rulebook"),
        ("rules", "severity"),
        ("overrides", "severity"),
        ("ignore_patterns", "check"),
        ("backend", "decision_model"),
    ]:
        json_change(f"old-{key}", CONFIG, (key,), {}, offending=key)
        # The public refusal must name both the old key and its replacement.
        cases[-1] = cases[-1].model_copy(update={"reason": replacement})
    keep(
        "config-duplicate",
        CONFIG,
        "marker_prefix",
        "duplicate key",
        lambda root: (root / CONFIG).write_text(
            (root / CONFIG)
            .read_text()
            .replace(
                '"marker_prefix": "K",',
                '"marker_prefix": "K", "marker_prefix": "K",',
                1,
            )
        ),
    )
    keep(
        "config-syntax",
        CONFIG,
        "",
        "malformed JSON",
        lambda root: (root / CONFIG).write_text('{"rulebook": [\n'),
    )
    keep(
        "marker-missing-entry",
        RULEBOOK,
        "K004",
        "marker without entry",
        lambda root: (root / RULEBOOK).write_text(
            (root / RULEBOOK).read_text() + "\nExtra standard. <!-- K004 -->\n"
        ),
    )
    keep(
        "entry-missing-marker",
        RULES,
        "K003",
        "entry without marker",
        lambda root: (root / RULEBOOK).write_text(
            (root / RULEBOOK).read_text().replace("<!-- K003 -->", "")
        ),
    )
    keep(
        "marker-duplicate",
        RULEBOOK,
        "K001",
        "duplicate marker",
        lambda root: (root / RULEBOOK).write_text(
            (root / RULEBOOK).read_text() + "\nAnother standard. <!-- K001 -->\n"
        ),
    )
    keep(
        "marker-forbidden-host",
        RULEBOOK,
        "K001",
        "forbidden marker host",
        lambda root: (root / RULEBOOK).write_text(
            (root / RULEBOOK)
            .read_text()
            .replace("Each workflow step", "> Each workflow step")
        ),
    )
    keep(
        "marker-empty-standard",
        RULEBOOK,
        "K001",
        "empty standard",
        lambda root: (root / RULEBOOK).write_text(
            (root / RULEBOOK)
            .read_text()
            .replace(
                "Each workflow step states a checkable completion covering its obligations. ",
                "",
            )
        ),
    )
    keep(
        "marker-over-word-limit",
        RULEBOOK,
        "K001",
        "standard over configured limit",
        lambda root: (root / RULEBOOK).write_text(
            (root / RULEBOOK)
            .read_text()
            .replace("Each workflow step", "word " * 401 + "Each workflow step")
        ),
    )
    json_change(
        "duplicate-rule-name",
        RULES,
        ("K003", "name"),
        "step_completion",
        offending="step_completion",
    )
    json_change("entry-unknown-key", RULES, ("K001", "unknown"), True)
    json_change(
        "undefined-kind",
        RULES,
        ("K001", "evidence"),
        "absent_kind",
        offending="absent_kind",
    )
    for name, field in [("reserved-field", "text"), ("invalid-field-name", "BadField")]:
        json_change(
            name,
            CONFIG,
            ("evidence", "kinds", "step", "fields", field),
            {"from": "first_line"},
            offending=field,
        )
    json_change(
        "unknown-kind-rule",
        CONFIG,
        ("evidence", "kinds", "step", "rule"),
        "absent_rule",
        offending="absent_rule",
    )
    json_change(
        "unknown-field-rule",
        CONFIG,
        ("evidence", "kinds", "step", "fields", "completion", "rule"),
        "absent_rule",
        offending="absent_rule",
    )
    json_change(
        "field-regex",
        CONFIG,
        ("evidence", "kinds", "step", "fields", "heading", "regex"),
        "[",
    )
    json_change(
        "kind-max-under-floor", CONFIG, ("evidence", "kinds", "step", "max_chars"), 999
    )
    json_change(
        "evidence-max-under-floor",
        CONFIG,
        ("limits",),
        {"evidence_max_chars": 999},
        offending="evidence_max_chars",
    )
    json_change(
        "as-kind-undefined",
        CONFIG,
        ("evidence", "kinds", "load", "fields", "targets", "as_kind"),
        "absent_kind",
        offending="as_kind",
    )
    json_change(
        "as-kind-has-target",
        CONFIG,
        ("evidence", "kinds", "load", "fields", "targets", "as_kind"),
        "load",
        offending="as_kind",
    )

    quality = ("K001", "decision", "steps", "quality")
    choice = ("K001", "decision", "steps", "kind")
    for name, path, value in [
        ("question-invalid", ("K003", "questions", "names_trigger", "type"), "invalid"),
        (
            "question-field-undefined",
            ("K003", "questions", "names_trigger", "instructions", "check"),
            "absent_field",
        ),
        (
            "choice-too-few",
            ("K001", "questions", "kind", "criteria"),
            {"outcome": "Only one option"},
        ),
        (
            "score-too-few",
            ("K001", "questions", "checkable", "criteria"),
            ["Only one level"],
        ),
        ("ask-undefined", choice + ("ask",), "absent_question"),
        ("combine-undefined", quality + ("combine",), {"absent_question": 1}),
        ("combine-choice", quality + ("combine",), {"kind": 1}),
        (
            "choice-unrouted",
            choice + ("routes",),
            {"outcome": "pass", "action": "pass"},
        ),
        ("choice-unknown-route", choice + ("routes", "unknown"), "pass"),
        ("weight-zero", quality + ("combine",), {"covers": 0, "checkable": 1}),
        ("weight-negative", quality + ("combine",), {"covers": -1, "checkable": 2}),
        ("weights-sum", quality + ("combine",), {"covers": 0.6, "checkable": 0.6}),
        ("threshold-negative", quality + ("no_at_or_below",), -0.1),
        ("threshold-over-one", quality + ("yes_at_or_above",), 1.1),
        ("threshold-equal", quality + ("yes_at_or_above",), 0.6),
        ("threshold-reversed", quality + ("yes_at_or_above",), 0.5),
        ("probability-zero", choice + ("min_probability",), 0),
        ("probability-over-one", choice + ("min_probability",), 1.1),
        ("start-undefined", ("K001", "decision", "start"), "absent_step"),
        ("step-undefined", quality + ("yes",), "absent_step"),
        ("cycle", quality + ("yes",), "has_completion"),
        ("nonterminal", quality + ("no",), {}),
        (
            "tool-unknown",
            ("K002", "decision", "steps", "under_300", "mechanical_check", "tool"),
            "absent_tool",
        ),
        (
            "tool-field-undefined",
            ("K002", "decision", "steps", "under_300", "mechanical_check", "field"),
            "absent_field",
        ),
        ("at-undefined", quality + ("no", "at"), "absent_field"),
    ]:
        json_change(name, RULES, path, value)
    for name, tool, field in [
        ("regex-target", "regex", "targets"),
        ("exists-text", "file_exists", "description"),
    ]:
        kind = "load" if field == "targets" else "skill"
        entry: JsonObject = {
            "name": "shape_check",
            "evidence": kind,
            "decision": {
                "start": "shape",
                "steps": {
                    "shape": {
                        "mechanical_check": {
                            "tool": tool,
                            "field": field,
                            **({"pattern": "."} if tool == "regex" else {}),
                        },
                        "yes": "pass",
                        "no": "pass",
                    }
                },
            },
        }
        json_change(name, RULES, ("K002",), entry, offending=field)
    keep(
        "threshold-nonfinite",
        RULES,
        "yes_at_or_above",
        "nonfinite number",
        lambda root: (root / RULES).write_text(
            (root / RULES)
            .read_text()
            .replace('"yes_at_or_above": 0.8', '"yes_at_or_above": 1e9999', 1)
        ),
    )
    for name, text, offending in [
        (
            "ast-rule-invalid",
            '{"id":"broken","language":"Markdown","rule":{"kind":"not_a_node"}}',
            "broken",
        ),
        (
            "non-markdown",
            '{"id":"foreign","language":"Python","rule":{"kind":"identifier"}}',
            "foreign",
        ),
        (
            "reserved-rule-id",
            '{"id":"quoted","language":"Markdown","rule":{"kind":"paragraph"}}',
            "quoted",
        ),
    ]:
        file = "evidence/rules/refusal.yml"
        keep(
            name,
            file,
            offending,
            name,
            lambda root, text=text, file=file: (root / file).write_text(text),
        )
    keep(
        "reserved-util-id",
        "evidence/utils/refusal.yml",
        "quoted",
        "reserved util",
        lambda root: (root / "evidence/utils/refusal.yml").write_text(
            '{"id":"quoted","language":"Markdown","rule":{"kind":"paragraph"}}'
        ),
    )
    keep(
        "ast-util-invalid",
        "evidence/utils/refusal.yml",
        "broken_util",
        "unknown utility",
        lambda root: (root / "evidence/utils/refusal.yml").write_text(
            '{"id":"broken_util","language":"Markdown","rule":{"matches":"absent_util"}}'
        ),
    )
    json_change(
        "project-unknown-util",
        "evidence/rules/skill-files.yml",
        ("rule", "matches"),
        "absent_util",
        offending="absent_util",
    )
    json_change(
        "missing-evidence-project",
        CONFIG,
        ("evidence", "ast_grep"),
        "absent.yml",
        offending="absent.yml",
    )
    json_change(
        "missing-rule-file",
        CONFIG,
        ("rulebook",),
        [{"markdown": RULEBOOK, "rules": "absent.json"}],
        offending="absent.json",
    )
    write_json(
        destination / "manifest.json", [case.model_dump(mode="json") for case in cases]
    )
    return cases


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--prepare", action="store_true")
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--source-root", type=Path, default=Path.cwd())
    parser.add_argument("--commit")
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
        cases = prepare_cases(evidence / "fixtures")
        if args.prepare:
            write_json(
                evidence / "prepared.json",
                {"product_observed": False, "cases": len(cases), "oracle": ORACLE},
            )
            print(f"Prepared {len(cases)} R24 controls; product unobserved")
            return
        if target is None:
            raise ValueError("Declared target missing")
        results: list[JsonObject] = []
        with RulebookProcess(target=target, evidence=evidence / "run") as process:
            clean = prepare_fixture(evidence / "clean")
            configure(
                clean,
                lambda config: object_value(config["decision_model"]).update(
                    endpoint=process.server.endpoint
                ),
            )
            out, err, calls = process.run(
                clean,
                "clean-check",
                ["lint", "--config", CONFIG, "--check"],
                expected_exit=0,
            )
            text = (out + err).decode()
            require(
                all(marker in text for marker in ("K001", "K002", "K003")),
                "Clean check lost rule listing",
            )
            require(not calls, "Clean check dispatched a request")
            for case in cases:
                root = evidence / "fixtures" / case.name
                # Keep duplicate and malformed JSON intact; parsing and writing
                # it here would erase the exact refusal this case must reach.
                config_path = root / CONFIG
                config_path.write_text(
                    config_path.read_text().replace(
                        "http://127.0.0.1:9/systemone", process.server.endpoint
                    )
                )
                out, err, calls = process.run(
                    root,
                    case.name,
                    ["lint", "--config", CONFIG, "--check"],
                    expected_exit=2,
                )
                text = (out + err).decode()
                require(not calls, f"{case.name}: check sent HTTP")
                require(
                    re.search(re.escape(Path(case.file).name) + r":\d+(?::\d+)?:", text)
                    is not None,
                    f"{case.name}: no located diagnostic",
                )
                require(
                    case.offending.lower() in text.lower(),
                    f"{case.name}: diagnostic lost {case.offending}",
                )
                if case.name.startswith("old-"):
                    require(
                        case.reason.lower() in text.lower(),
                        f"{case.name}: replacement not named",
                    )
                results.append({"case": case.name, "exit": 2, "http_requests": 0})
        write_json(
            evidence / "pass.json",
            {
                "commit": target.commit,
                "binary_sha256": target.digest,
                "cases": results,
                "oracle": ORACLE,
                "required_join": "crates/sunstack-lint/tests/rulebook_lint_gates.rs: recording reads and backend construction; this process driver alone does not prove those",
            },
        )
        print(
            f"Observed {len(results)} refusal cases; recording-source join remains separate"
        )
    except (ValueError, TypeError, OSError, subprocess.SubprocessError) as error:
        parser.exit(2, f"Rulebook check observation failed: {error}\n")


if __name__ == "__main__":
    main()
