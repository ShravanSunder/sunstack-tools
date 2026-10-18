# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic==2.14.1", "pydantic_core==2.50.1"]
# ///
"""Assemble three marked real teaching standards and check their printed bytes.

Run from the monorepo root with uv run --script <this-file> --binary <binary>
--commit <full-HEAD> --evidence <new-owned-directory>. Requires check_rules (S2).
Uses local Git fixture commits/clones, never a network clone or provider. Keeps
two same-commit checkouts, inputs, outputs, exits and literal question oracles.
Oracle: Markdown-rule Specification R6/R8–R13, Verification Design U1/U2 and
the deterministic-print portion of U3. Change-scope selection is separate proof.
"""

import copy
import hashlib
import shutil
import subprocess
from pathlib import Path

# Executed by file path: this directory is Python's script import root.
from rule_source_runtime import (  # pyright: ignore[reportImplicitRelativeImport]
    FIXTURES,
    JsonObject,
    as_array,
    as_object,
    journey_target,
    object_at,
    printed_rule_definitions,
    read_json,
    rule_entry,
    source_fixture,
    write_json,
)

QUESTION_FILES = {
    "skill.call.complete": "call-grammar.question.txt",
    "skill.step.completion": "completion-criteria.question.txt",
    "skill.description.good": "good-description.question.txt",
}


def native_definitions(outcome: JsonObject) -> dict[str, JsonObject]:
    definitions = printed_rule_definitions(outcome)
    assert set(definitions) == set(QUESTION_FILES)
    return definitions


def question_bytes(definitions: dict[str, JsonObject]) -> JsonObject:
    return {
        name: object_at(definition, "verdict", "question")["instructions"]
        for name, definition in definitions.items()
    }


def require_questions(definitions: dict[str, JsonObject], expected: JsonObject) -> None:
    assert question_bytes(definitions) == expected, (
        "Printed bytes differ from visible-text literals"
    )
    for name, definition in definitions.items():
        assert definition["target"] == "target"
        assert definition["applicability"] == [] and definition["questions"] == {}
        assert definition["severity"] == (
            "block" if name == "skill.call.complete" else "nudge"
        )
        verdict = object_at(definition, "verdict")
        assert verdict["kind"] == "noul" and verdict["threshold"] == 0.8
        evidence = as_array(definition["evidence"])
        assert len(evidence) == 1
        target = as_object(evidence[0])
        assert target["id"] == "target" and target["input"] == "subject"
        assert target["select"] == {"kind": "paragraph", "target_id": "U001"}


def git_fixture(root: Path, *arguments: str) -> str:
    completed = subprocess.run(
        [
            "git",
            "-c",
            "user.name=Verification",
            "-c",
            "user.email=verification@example.invalid",
            "-C",
            str(root),
            *arguments,
        ],
        capture_output=True,
        text=True,
        check=True,
        timeout=30,
    )
    return completed.stdout.strip()


def main() -> None:
    target = journey_target(__doc__ or "Assemble and print rule sources")
    provenance = read_json(FIXTURES / "source-provenance.json")
    for source in as_array(provenance["sources"]):
        item = as_object(source)
        filename = item["file"]
        assert isinstance(filename, str)
        assert (
            hashlib.sha256((FIXTURES / filename).read_bytes()).hexdigest()
            == item["marked_sha256"]
        )
    expected: JsonObject = {
        name: (FIXTURES / filename).read_text().removesuffix("\n")
        for name, filename in QUESTION_FILES.items()
    }
    root = target.evidence / "fixture-origin"
    rules: JsonObject = {
        "K014": {**rule_entry("skill.call.complete"), "severity": "block"},
        "K015": rule_entry("skill.step.completion"),
    }
    invocation = source_fixture(
        root, (FIXTURES / "skill-creation.md").read_text(), rules
    )
    _ = shutil.copyfile(
        FIXTURES / "frontmatter-design.md", root / "frontmatter-design.md"
    )
    write_json(
        root / "frontmatter.rules.json", {"K016": rule_entry("skill.description.good")}
    )
    ruleset = read_json(root / "set.json")
    ruleset["max_question_words"] = 1000
    as_array(ruleset["rule_sources"]).append(
        {"markdown": "frontmatter-design.md", "rules": "frontmatter.rules.json"}
    )
    write_json(root / "set.json", ruleset)
    _ = git_fixture(root, "init", "--initial-branch=main", "--template=")
    _ = git_fixture(
        root,
        "add",
        "source.md",
        "source.rules.json",
        "frontmatter-design.md",
        "frontmatter.rules.json",
        "set.json",
    )
    _ = git_fixture(
        root, "commit", "--no-gpg-sign", "-m", "Verification fixture baseline"
    )
    fixture_commit = git_fixture(root, "rev-parse", "HEAD")
    baseline = native_definitions(
        target.run("origin-first", "check_rules", invocation, expected_exit=0)
    )
    require_questions(baseline, expected)
    repeated = native_definitions(
        target.run("origin-repeat", "check_rules", invocation, expected_exit=0)
    )
    assert repeated == baseline
    for index in [1, 2]:
        checkout = target.evidence / f"fixture-checkout-{index}"
        _ = git_fixture(
            target.evidence,
            "clone",
            "--no-hardlinks",
            "--template=",
            str(root),
            str(checkout),
        )
        assert git_fixture(checkout, "rev-parse", "HEAD") == fixture_commit
        cloned_input = copy.deepcopy(invocation)
        cloned_input["definition_root"] = str(checkout)
        as_object(as_array(cloned_input["repositories"])[0])["root"] = str(checkout)
        cloned = native_definitions(
            target.run(
                f"checkout-{index}", "check_rules", cloned_input, expected_exit=0
            )
        )
        assert cloned == baseline, (
            "Same committed sources changed assembled definitions"
        )

    source = root / "source.md"
    original = source.read_text()
    before = "Test every one twice:"
    after = "Test each criterion twice:"
    assert original.count(before) == 1
    _ = source.write_text(original.replace(before, after))
    changed = native_definitions(
        target.run("teaching-edit", "check_rules", invocation, expected_exit=0)
    )
    changed_expected = dict(expected)
    old_question = changed_expected["skill.step.completion"]
    assert isinstance(old_question, str)
    changed_expected["skill.step.completion"] = old_question.replace(before, after)
    require_questions(changed, changed_expected)
    assert changed["skill.call.complete"] == baseline["skill.call.complete"]
    assert changed["skill.description.good"] == baseline["skill.description.good"]

    _ = source.write_text(original)
    as_object(rules["K015"])["threshold"] = 0.77
    write_json(root / "source.rules.json", rules)
    configured = native_definitions(
        target.run("config-edit", "check_rules", invocation, expected_exit=0)
    )
    assert question_bytes(configured) == expected
    assert (
        object_at(configured["skill.step.completion"], "verdict")["threshold"] == 0.77
    )
    target.confirm()
    write_json(
        target.evidence / "journey-pass.json",
        {
            "journey": "assemble and print rule sources",
            "commit": target.commit,
            "fixture_commit": fixture_commit,
            "unchanged_print_runs": 4,
            "teaching_edit_changes_only_its_question": True,
            "config_edit_changes_threshold_without_question": True,
            "change_scope_selection": "separate U3 integration-test obligation",
        },
    )
    print(
        f"assemble and print: 3 standards, 6 processes passed; evidence {target.evidence}"
    )


if __name__ == "__main__":
    main()
