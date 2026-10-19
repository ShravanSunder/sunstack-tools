"""Build S1 scenario cases and their independent expected-results files."""

import copy
import json
from pathlib import Path

from runner_contracts import (
    FixtureCase,
    FixtureManifest,
    JsonObject,
    as_object,
    write_json,
)
from runner_fixture_setup import (
    BAD_TEXT,
    GOOD_TEXT,
    base_config,
    expected_finding,
    git_command,
    judged_rule,
    mechanical_rule,
    write_fixture,
)


def build_fixtures(
    destination: Path, endpoint: str = "http://127.0.0.1:9/systemone"
) -> FixtureManifest:
    destination.mkdir(parents=True, exist_ok=False)
    cases: list[FixtureCase] = []

    def keep(
        name: str,
        *,
        config: JsonObject | None = None,
        rules: list[JsonObject] | None = None,
        git: bool = False,
        **fields: object,
    ) -> Path:
        root = destination / name
        resolved = copy.deepcopy(config if config is not None else base_config())
        if "backend" in resolved:
            as_object(resolved["backend"])["endpoint"] = endpoint
        write_fixture(root, config=resolved, rules=rules, git=git)
        case = FixtureCase.model_validate(
            {
                "name": name,
                "findings": [expected_finding().model_dump(mode="json")],
                "wrong_build": "A mechanical error exits clean or disappears",
                **fields,
            }
        )
        cases.append(case)
        return root

    keep("mechanical-error", status="findings")
    root = keep("clean", exit_code=0, findings=[], status="clean")
    (root / "docs/one.md").write_text(GOOD_TEXT)
    keep(
        "default-warn",
        rules=[mechanical_rule(None)],
        exit_code=0,
        findings=[expected_finding("warn").model_dump(mode="json")],
    )
    keep(
        "warning-limit",
        rules=[mechanical_rule("warn")],
        arguments=["lint", "--all", "--format", "json", "--max-warnings", "0"],
        findings=[expected_finding("warn").model_dump(mode="json")],
    )

    for name, authored, settings, overrides, severity in [
        ("entry-wildcard", "warn", {"docs/*": "error"}, [], "error"),
        (
            "exact-wins",
            "error",
            {"docs/*": "error", "docs/standard": "warn"},
            [],
            "warn",
        ),
        (
            "ordered-overrides",
            "error",
            {},
            [
                {"files": ["docs/*"], "rules": {"docs/*": "error"}},
                {"files": ["docs/*"], "rules": {"docs/*": "warn"}},
            ],
            "warn",
        ),
        (
            "override-exact-wins",
            "warn",
            {},
            [
                {
                    "files": ["docs/*"],
                    "rules": {"docs/*": "warn", "docs/standard": "error"},
                }
            ],
            "error",
        ),
        ("off", "error", {"docs/*": "off"}, [], "off"),
    ]:
        config = base_config()
        config.update(rules=settings, overrides=overrides)
        keep(
            name,
            config=config,
            rules=[mechanical_rule(authored)],
            exit_code=1 if severity == "error" else 0,
            findings=[]
            if severity == "off"
            else [expected_finding(severity).model_dump(mode="json")],
            wrong_build="R10 precedence changes or off creates a finding",
        )

    for name in (
        "zero-candidates",
        "clean-outside-scope",
        "named-clean",
        "changed-rule",
        "changed-ruleset",
        "unrelated-edit",
        "changed-config",
        "staged-target",
        "unstaged-target",
        "untracked-target",
        "since-target",
        "all-clean",
    ):
        no_selection = name in (
            "zero-candidates",
            "clean-outside-scope",
            "unrelated-edit",
        )
        arguments = ["lint", "--format", "json"]
        if name == "named-clean":
            arguments.append("docs/one.md")
        if name in ("all-clean", "zero-candidates"):
            arguments.append("--all")
        root = keep(
            name,
            git=True,
            arguments=arguments,
            exit_code=0 if no_selection else 1,
            findings=[]
            if no_selection
            else [expected_finding().model_dump(mode="json")],
            status="nothing_selected" if no_selection else "findings",
            exclusion="outside_change_scope"
            if name in ("clean-outside-scope", "unrelated-edit")
            else None,
            widened=name == "changed-config",
            wrong_build="R4 scope misses dependencies or selects unaffected clean documents",
        )
        baseline = git_command(root, "rev-parse", "HEAD")
        if name == "zero-candidates":
            config = base_config()
            as_object(as_object(config["rule_sets"])["docs"])["files"] = ["absent/*.md"]
            write_json(root / "sunstack.lint.json", config)
        elif name == "changed-rule":
            value = mechanical_rule()
            as_object(value["verdict"])["explanation"] = (
                "Each paragraph MUST obey this standard."
            )
            (root / "rules/rules.jsonl").write_text(json.dumps(value) + "\n")
        elif name == "changed-ruleset":
            value = json.loads((root / "rules/set.json").read_text())
            value["common_ruleset_instructions"] = "NEW_RULESET_CONTEXT"
            write_json(root / "rules/set.json", value)
        elif name == "unrelated-edit":
            (root / "unrelated.txt").write_text("An unrelated edit.\n")
        elif name == "changed-config":
            config = base_config()
            config["rules"] = {"docs/*": "error"}
            write_json(root / "sunstack.lint.json", config)
        elif name in ("staged-target", "unstaged-target", "since-target"):
            (root / "docs/one.md").write_text("You may skip this changed standard.\n")
            if name != "unstaged-target":
                git_command(root, "add", "docs/one.md")
            if name == "since-target":
                git_command(root, "commit", "-qm", "Changed target after baseline")
                cases[-1] = cases[-1].model_copy(
                    update={"arguments": [*arguments, "--since", baseline]}
                )
        elif name == "untracked-target":
            (root / "docs/two.md").write_text(BAD_TEXT)
            cases[-1] = cases[-1].model_copy(
                update={"findings": [expected_finding(file="docs/two.md")]}
            )
        write_json(
            root / "git-history.json",
            {
                "baseline": baseline,
                "head": git_command(root, "rev-parse", "HEAD"),
                "status": git_command(root, "status", "--porcelain"),
                "tracked_blob": git_command(root, "rev-parse", "HEAD:docs/one.md"),
            },
        )

    for name, text in (
        ("ignore-pattern", "docs/ignored.md\n"),
        ("git-ignored", "docs/ignored.md\n"),
    ):
        config = base_config()
        config["ignore_patterns"] = (
            ["docs/ignored.md"] if name == "ignore-pattern" else []
        )
        root = keep(name, config=config, git=True)
        (root / "docs/ignored.md").write_text(BAD_TEXT)
        if name == "git-ignored":
            (root / ".gitignore").write_text(text)
        cases[-1] = cases[-1].model_copy(
            update={
                "arguments": ["lint", "--all", "--format", "json"],
                "exclusion": "ignored",
            }
        )

    for name, mutate, diagnostic in [
        ("unknown-field", {"unknown": True}, "unknown_field"),
        (
            "missing-ruleset",
            {"rule_sets": {"docs": {"path": "missing.json", "files": ["docs/*"]}}},
            "missing_rule_set",
        ),
        (
            "same-ruleset",
            {
                "rule_sets": {
                    "docs": {"path": "rules/set.json", "files": ["docs/*"]},
                    "other": {"path": "rules/set.json", "files": ["docs/*"]},
                }
            },
            "same_rule_set_twice",
        ),
        ("unknown-selector", {"rules": {"docs/unknown": "off"}}, "unknown_selector"),
        ("bad-glob", {"ignore_patterns": ["["]}, "bad_glob"),
    ]:
        config = base_config()
        config.update(mutate)
        keep(
            name,
            config=config,
            arguments=["lint", "--check"],
            mode="check",
            exit_code=2,
            findings=[],
            diagnostic=diagnostic,
            wrong_build="Invalid config is accepted or loses file/line/column",
        )
    for word, replacement in (("block", "error"), ("nudge", "warn")):
        keep(
            f"old-{word}",
            rules=[mechanical_rule(word)],
            arguments=["lint", "--check"],
            mode="check",
            exit_code=2,
            findings=[],
            includes=[replacement],
            wrong_build="Old severity vocabulary remains accepted",
        )
    root = keep(
        "syntax",
        arguments=["lint", "--check"],
        mode="check",
        exit_code=2,
        findings=[],
        diagnostic="syntax",
    )
    (root / "sunstack.lint.json").write_text('{\n  "rule_sets": [\n}\n')
    root = keep(
        "duplicate-key",
        arguments=["lint", "--check"],
        mode="check",
        exit_code=2,
        findings=[],
        diagnostic="duplicate_key",
    )
    (root / "sunstack.lint.json").write_text(
        '{"rule_sets":{"docs":{"path":"rules/set.json","files":["docs/*"]},"docs":{"path":"rules/set.json","files":["docs/*"]}}}\n'
    )
    root = keep(
        "unsupported-input",
        arguments=["lint", "--check"],
        mode="check",
        exit_code=2,
        findings=[],
        diagnostic="unsupported_inputs",
    )
    rule_set = json.loads((root / "rules/set.json").read_text())
    rule_set["inputs"]["extra"] = {"kind": "document"}
    write_json(root / "rules/set.json", rule_set)
    keep(
        "missing-path",
        arguments=["lint", "--all", "absent.md", "--format", "json"],
        exit_code=2,
        findings=[],
        status="invalid",
        diagnostic="path_not_found",
    )
    keep(
        "outside-git",
        arguments=["lint", "--format", "json"],
        exit_code=2,
        findings=[],
        status="invalid",
        diagnostic="no_git_for_default_scope",
        includes=["--all"],
    )
    root = keep(
        "missing-config",
        arguments=["lint", "--format", "json"],
        exit_code=2,
        findings=[],
        status="invalid",
        diagnostic="no_config_found",
        includes=["sunstack.lint.json"],
    )
    (root / "sunstack.lint.json").unlink()
    root = keep(
        "nearest-config",
        git=True,
        cwd="nested",
        selected_config="nested/sunstack.lint.json",
    )
    (root / "nested").mkdir()
    config = base_config()
    as_object(as_object(config["rule_sets"])["docs"])["path"] = "../rules/set.json"
    as_object(as_object(config["rule_sets"])["docs"])["files"] = ["../docs/*.md"]
    write_json(root / "nested/sunstack.lint.json", config)
    root = keep(
        "explicit-config",
        cwd="nested",
        arguments=[
            "lint",
            "--all",
            "--config",
            "../sunstack.lint.json",
            "--format",
            "json",
        ],
    )
    (root / "nested").mkdir()
    (root / "nested/sunstack.lint.json").write_text("{}\n")
    root = keep(
        "check-unreadable-target",
        arguments=["lint", "--check"],
        mode="check",
        exit_code=0,
        findings=[],
        unreadable="docs/one.md",
        wrong_build="--check reads a selected target",
    )
    for name, file in (("rules-all", None), ("rules-file", "docs/one.md")):
        config = base_config()
        config.update(
            rules={"docs/*": "off"},
            overrides=[{"files": ["docs/*"], "rules": {"docs/standard": "warn"}}],
        )
        keep(
            name,
            config=config,
            arguments=["lint", "--rules", *([file] if file else [])],
            mode="rules",
            exit_code=0,
            findings=[],
            rules_file=file,
            rules_setting="warn" if file else "off",
            rules_rule="docs/standard",
            rules_source_file="rules/rules.jsonl",
            rules_source_line=1,
            rules_origin={"kind": "override", "index": 0} if file else None,
            wrong_build="--rules omits off rules or misreports deciding override",
        )
    root = destination / "init"
    root.mkdir()
    cases.append(
        FixtureCase(
            name="init",
            arguments=["lint", "--init"],
            mode="init",
            exit_code=0,
            wrong_build="--init writes wrong skeleton or overwrites",
        )
    )

    for name, kind, severity, override, probability, band in [
        ("judged-error", "answer", "error", None, 0.1, None),
        ("judged-warn", "answer", "warn", None, 0.1, None),
        ("judged-promote", "answer", "warn", "error", 0.1, None),
        ("judged-demote", "answer", "error", "warn", 0.1, None),
        ("judged-band-inclusive", "answer", "error", None, 0.3, 0.3),
        ("judged-above-band", "answer", "error", "error", 0.31, 0.3),
        ("judged-at-threshold", "answer", "error", None, 0.8, 0.3),
        ("judged-no-cost", "answer", "error", None, 0.1, None),
        ("judged-off", "off", "error", "off", 0.1, None),
        ("judged-no-key", "no-key", "error", None, 0.1, None),
        ("judged-no-backend", "no-backend", "error", None, 0.1, None),
        ("cancel", "cancel", "error", None, 0.1, None),
    ]:
        config = base_config()
        if kind == "no-backend":
            config.pop("backend")
        if kind != "no-backend":
            config["backend"] = {
                "kind": "jev_openrouter",
                "model": "jev-latest",
                "endpoint": "http://127.0.0.1:9/systemone",
            }
        if override:
            config["rules"] = {"docs/*": override}
        effective = (
            "warn"
            if override == "warn"
            or (severity == "warn" and override != "error")
            or band is not None
            and band < probability < 0.8
            else "error"
        )
        selected = kind == "answer" and probability < 0.8
        root = keep(
            name,
            config=config,
            rules=[judged_rule(severity, band)],
            credential=kind in ("answer", "cancel"),
            requests=1 if kind in ("answer", "cancel") else 0,
            findings=[
                expected_finding(effective, kind="judged").model_dump(mode="json")
            ]
            if selected
            else [],
            probability=probability,
            cost=name != "judged-no-cost",
            cancel=kind == "cancel",
            incomplete=False,
            exit_code=4
            if kind == "cancel"
            else 2
            if kind in ("no-key", "no-backend")
            else 1
            if selected and effective == "error"
            else 0,
            diagnostic="credential_missing"
            if kind == "no-key"
            else "backend_missing"
            if kind == "no-backend"
            else None,
            wrong_build="Judged severity/band/key/off/request-evidence behavior violates R10/R13",
        )
        (root / "docs/one.md").write_text("U001: Two plus two equals five.\n")
    root = keep(
        "error-plus-incomplete",
        rules=[
            mechanical_rule(),
            {
                **mechanical_rule(),
                "rule_id": "unavailable",
                "evidence": [
                    {
                        "id": "target",
                        "input": "subject",
                        "select": {"kind": "frontmatter_field", "field": "description"},
                    }
                ],
            },
        ],
        exit_code=3,
        incomplete=True,
        status="incomplete",
        wrong_build="Error finding masks incomplete part",
    )

    for name, enabled in (
        ("deterministic-gate-pass", True),
        ("deterministic-gate-stop", False),
    ):
        rule = mechanical_rule()
        rule["applicability"] = [
            {
                "kind": "condition",
                "test": {
                    "kind": "pattern_present",
                    "input": "subject",
                    "pattern": {
                        "kind": "literal",
                        "text": "may" if enabled else "ABSENT_MARKER",
                    },
                    "max_scan_bytes": 10000,
                },
            }
        ]
        keep(
            name,
            rules=[rule],
            exit_code=1 if enabled else 0,
            findings=[expected_finding().model_dump(mode="json")] if enabled else [],
            wrong_build="Deterministic gates require a key or fail to exclude work",
        )
    gate = {
        "kind": "decision",
        "question": {
            "type": "noul",
            "instructions": "Enable the supplied synthetic standard.",
        },
        "evidence": [
            {"id": "gate", "input": "subject", "select": {"kind": "whole_document"}}
        ],
        "threshold": 0.6,
    }
    for name, off in (("model-gate-no-key", False), ("off-only-model-gate", True)):
        rule = mechanical_rule()
        rule["applicability"] = [gate]
        config = base_config()
        if off:
            config["rules"] = {"docs/*": "off"}
        keep(
            name,
            config=config,
            rules=[rule],
            exit_code=0 if off else 2,
            findings=[],
            diagnostic=None if off else "credential_missing",
            wrong_build="An off-only model gate costs a request, or reachable model gate misses key refusal",
        )
    keep(
        "check-judged-unreadable",
        rules=[judged_rule()],
        arguments=["lint", "--check"],
        mode="check",
        exit_code=0,
        findings=[],
        unreadable="docs/one.md",
        wrong_build="--check reads targets or asks for model credentials",
    )
    old_band = judged_rule(band=0.3)
    as_object(old_band["verdict"])["block_at_or_below"] = as_object(
        old_band["verdict"]
    ).pop("error_at_or_below")
    keep(
        "old-band-field",
        rules=[old_band],
        arguments=["lint", "--check"],
        mode="check",
        exit_code=2,
        findings=[],
        includes=["error_at_or_below"],
        wrong_build="Old band field is accepted or replacement diagnostic is absent",
    )
    format_rule = judged_rule()
    format_rule["rule_id"] = "judgment"
    root = keep(
        "mixed-formats",
        rules=[mechanical_rule(), format_rule],
        credential=True,
        requests=1,
        findings=[
            expected_finding().model_dump(mode="json"),
            expected_finding(rule="judgment", kind="judged").model_dump(mode="json"),
        ],
        wrong_build="One output format drops a mechanical or judged finding",
    )
    (root / "docs/one.md").write_text("U001: Two plus two equals five.\n")

    for case in cases:
        write_json(
            destination / case.name / "expected-results.json",
            case.model_dump(mode="json"),
        )
    manifest = FixtureManifest(
        cases=cases,
        gaps=[
            "R15 schema agreement awaits published schema and exact-target native proof.",
            "R7 Rust library equality reuses Implementer's real integration test; not authored here.",
            "Model gates/off-only/shared-enabled-peer semantics additionally require native RulePolicy proof.",
        ],
    )
    write_json(destination / "manifest.json", manifest.model_dump(mode="json"))
    return manifest
