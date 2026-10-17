"""Own synthetic Git repositories without inheriting the owner's Git execution policy."""

import os
import subprocess
import typing as t
from dataclasses import dataclass
from pathlib import Path

from scenario_fixtures import base_invocation, bindings, noul_rule, ruleset
from scenario_runtime import REPOSITORY_ROOT, JsonObject, object_at, write_object

IDENTITY = "Scenario Fixture"
EMAIL = "fixture@example.invalid"
BASE_DOCUMENT = "U001: original\n\nU002: neighbor\n"


@dataclass(kw_only=True)
class FixtureRepository:
    root: Path
    evidence: Path
    inherited: t.Mapping[str, str] | None = None

    def command(
        self, arguments: list[str], timestamp: int
    ) -> tuple[list[str], dict[str, str]]:
        assert self.root.resolve().is_relative_to(self.evidence.resolve())
        hooks = self.evidence / "fixture-git-empty-hooks"
        template = self.evidence / "fixture-git-empty-template"
        for directory in [hooks, template]:
            directory.mkdir(parents=True, exist_ok=True)
            assert directory.resolve() == directory.absolute()
            assert not list(directory.iterdir()), (
                "Fixture hooks/templates must be empty"
            )
        inherited = os.environ if self.inherited is None else self.inherited
        environment = {
            key: value
            for key, value in inherited.items()
            if not key.startswith("GIT_") and key != "OPENROUTER_API_KEY"
        }
        environment.update(
            {
                "GIT_CONFIG_GLOBAL": "/dev/null",
                "GIT_CONFIG_NOSYSTEM": "1",
                "GIT_AUTHOR_NAME": IDENTITY,
                "GIT_AUTHOR_EMAIL": EMAIL,
                "GIT_COMMITTER_NAME": IDENTITY,
                "GIT_COMMITTER_EMAIL": EMAIL,
                "GIT_AUTHOR_DATE": f"@{timestamp} +0000",
                "GIT_COMMITTER_DATE": f"@{timestamp} +0000",
            }
        )
        command = ["git", "--no-optional-locks"]
        for setting in [
            "commit.gpgsign=false",
            "tag.gpgsign=false",
            f"user.name={IDENTITY}",
            f"user.email={EMAIL}",
            f"core.hooksPath={hooks}",
            f"init.templateDir={template}",
        ]:
            command.extend(["-c", setting])
        command.extend(["-C", str(self.root), *arguments])
        return command, environment

    def check_contract(
        self, command: list[str], environment: t.Mapping[str, str], timestamp: int
    ) -> None:
        expected = {
            "GIT_CONFIG_GLOBAL": "/dev/null",
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_AUTHOR_NAME": IDENTITY,
            "GIT_AUTHOR_EMAIL": EMAIL,
            "GIT_COMMITTER_NAME": IDENTITY,
            "GIT_COMMITTER_EMAIL": EMAIL,
            "GIT_AUTHOR_DATE": f"@{timestamp} +0000",
            "GIT_COMMITTER_DATE": f"@{timestamp} +0000",
        }
        assert {
            key: value for key, value in environment.items() if key.startswith("GIT_")
        } == expected
        assert command[:2] == ["git", "--no-optional-locks"]
        for setting in [
            "commit.gpgsign=false",
            "tag.gpgsign=false",
            f"user.name={IDENTITY}",
            f"user.email={EMAIL}",
            f"core.hooksPath={self.evidence / 'fixture-git-empty-hooks'}",
            f"init.templateDir={self.evidence / 'fixture-git-empty-template'}",
        ]:
            assert setting in command and command[command.index(setting) - 1] == "-c"
        position = command.index("-C")
        assert command[position + 1] == str(self.root)
        if command[position + 2] == "commit":
            assert "--no-gpg-sign" in command[position + 3 :]

    def git(self, *arguments: str, timestamp: int = 1000) -> str:
        command, environment = self.command(list(arguments), timestamp)
        self.check_contract(command, environment, timestamp)
        return subprocess.run(
            command,
            cwd=REPOSITORY_ROOT,
            env=environment,
            check=True,
            capture_output=True,
            text=True,
            timeout=15,
        ).stdout.strip()

    def initialize(self) -> None:
        self.root.mkdir(parents=True, exist_ok=False)
        self.git("init", "--initial-branch=scenario")

    def write(self, relative: str, content: str) -> None:
        path = self.root / relative
        assert path.resolve().is_relative_to(self.root.resolve())
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)

    def commit(self, timestamp: int = 1000) -> str:
        self.git("add", "--all")
        self.git(
            "commit", "--no-gpg-sign", "-m", "synthetic scenario", timestamp=timestamp
        )
        return self.git("rev-parse", "HEAD")

    def binding(self, key: str, scope: JsonObject) -> JsonObject:
        return {
            "key": key,
            "root": str(self.root),
            "input_paths": [{"kind": "glob", "pattern": "**"}],
            "change_scope": scope,
        }


def discovery() -> JsonObject:
    return {
        "kind": "bounded",
        "limits": {
            "max_files": 100,
            "max_file_bytes": 100000,
            "max_total_bytes": 1000000,
            "max_matches": 1000,
            "include_ignored": False,
            "max_unit_visits": 1000,
            "max_relationship_candidates": 1000,
            "max_fan_out": 100,
            "max_history_commits": 100,
            "max_local_time_ms": 10000,
        },
    }


def related_selection() -> JsonObject:
    return {
        "kind": "related_units",
        "languages": ["typescript"],
        "unit_kinds": ["function"],
        "source_sides": "after_only",
        "directions": ["callers"],
        "requested_hops": 1,
        "chunk_settings": {
            "grouping": "single_group",
            "seed_content": {"kind": "whole_unit"},
            "related_content": {"kind": "whole_unit"},
            "max_chunk_bytes": 10000,
        },
        "minimum_related_units": 1,
    }


def paragraph_fixture(
    root: Path, endpoint: str
) -> tuple[FixtureRepository, JsonObject]:
    repository = FixtureRepository(root=root, evidence=root.parent)
    repository.initialize()
    repository.write("source.md", BASE_DOCUMENT)
    definition = ruleset(["changed.jsonl", "neighbor.jsonl"])
    definition["inputs"] = {"subject": {"kind": "document"}}
    write_object(root / "set.json", definition)
    write_object(root / "changed.jsonl", noul_rule("changed"))
    neighbor = noul_rule("neighbor")
    evidence = neighbor["evidence"]
    assert isinstance(evidence, list)
    assert isinstance(evidence[0], dict)
    object_at(evidence[0], "select")["target_id"] = "U002"
    write_object(root / "neighbor.jsonl", neighbor)
    repository.commit()
    invocation = base_invocation(root, endpoint)
    bindings(invocation).pop("other")
    invocation["repositories"] = [
        repository.binding("repo", {"kind": "uncommitted", "layers": ["unstaged"]})
    ]
    invocation["discovery"] = discovery()
    object_at(invocation, "limits")["max_files"] = 100
    return repository, invocation


def fixture_git_checks(evidence: Path) -> None:
    directory = evidence / "fixture-git-control"
    directory.mkdir()
    marker = directory / "sentinel-ran"
    sentinel = directory / "sentinel"
    sentinel.write_text(f"#!/bin/sh\n/usr/bin/touch '{marker}'\nexit 42\n")
    sentinel.chmod(0o700)
    hostile_hooks = directory / "hostile-hooks"
    hostile_hooks.mkdir()
    (hostile_hooks / "pre-commit").write_bytes(sentinel.read_bytes())
    (hostile_hooks / "pre-commit").chmod(0o700)
    hostile = directory / "hostile.config"
    hostile.write_text(
        f"[commit]\n gpgsign = true\n[gpg]\n program = {sentinel}\n[core]\n hooksPath = {hostile_hooks}\n"
    )
    inherited = dict(os.environ)
    inherited.update(
        {
            "GIT_CONFIG_GLOBAL": str(hostile),
            "GIT_CONFIG_SYSTEM": str(hostile),
            "GIT_CONFIG_COUNT": "1",
            "GIT_CONFIG_KEY_0": "commit.gpgsign",
            "GIT_CONFIG_VALUE_0": "true",
            "GIT_DIR": str(directory / "wrong-repository"),
            "GIT_TEMPLATE_DIR": str(hostile_hooks),
            "GIT_AUTHOR_NAME": "wrong author",
        }
    )
    repository = FixtureRepository(
        root=directory / "repository", evidence=directory, inherited=inherited
    )
    repository.initialize()
    with (repository.root / ".git/config").open("a") as stream:
        stream.write(hostile.read_text())
    repository.write("source.md", "SYNTHETIC_GIT_CONTROL\n")
    repository.commit(1200)
    raw_commit = repository.git("cat-file", "-p", "HEAD")
    assert not marker.exists(), "Fixture Git invoked a signing/hook sentinel"
    assert "\ngpgsig " not in raw_commit
    assert f"author {IDENTITY} <{EMAIL}> 1200 +0000" in raw_commit
    assert f"committer {IDENTITY} <{EMAIL}> 1200 +0000" in raw_commit
    command, environment = repository.command(
        ["commit", "--no-gpg-sign", "-m", "control"], 1200
    )
    refused = 0
    for key in [key for key in environment if key.startswith("GIT_")]:
        malformed = dict(environment)
        malformed.pop(key)
        try:
            repository.check_contract(command, malformed, 1200)
        except AssertionError:
            refused += 1
    for setting in [
        item
        for index, item in enumerate(command)
        if index and command[index - 1] == "-c"
    ] + ["--no-gpg-sign"]:
        malformed_command = list(command)
        malformed_command.remove(setting)
        try:
            repository.check_contract(malformed_command, environment, 1200)
        except AssertionError:
            refused += 1
    assert refused == 15
    write_object(
        evidence / "fixture-git-checks.json",
        {
            "sentinel_exists": marker.exists(),
            "unsigned_commit": "\ngpgsig " not in raw_commit,
            "omitted_clauses_refused": refused,
            "commit": repository.git("rev-parse", "HEAD"),
            "raw_commit": raw_commit,
        },
    )
