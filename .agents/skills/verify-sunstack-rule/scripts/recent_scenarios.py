"""US20: preserve two-repository time-window and union meaning through the CLI."""

import copy
import hashlib
import os
from dataclasses import dataclass
from datetime import UTC, datetime, timedelta
from pathlib import Path

from git_scenario_fixtures import FixtureRepository, discovery
from scenario_fixtures import evidence_plan
from scenario_runtime import (
    DriverContext,
    JsonObject,
    OperationRequest,
    array_at,
    as_object,
    as_text,
    object_at,
    run_operation,
    write_object,
)
from scenario_transport import ControlledServer


@dataclass(frozen=True, kw_only=True)
class WindowObservation:
    outcome: JsonObject
    window: JsonObject
    launched: datetime
    finished: datetime


def utc_text(value: datetime) -> str:
    return value.isoformat().replace("+00:00", "Z")


def check_windows(observation: WindowObservation) -> None:
    reports = [
        as_object(value) for value in array_at(observation.outcome, "change_selections")
    ]
    assert len(reports) == 2
    anchors = [
        datetime.fromisoformat(as_text(report["anchor_utc"])) for report in reports
    ]
    assert anchors[0] == anchors[1]
    assert observation.launched <= anchors[0] <= observation.finished
    for report in reports:
        for value in array_at(report, "members"):
            bounds = object_at(as_object(value), "bounds")
            if bounds["kind"] not in ["recent_commits", "recent_uncommitted"]:
                continue
            since = datetime.fromisoformat(as_text(bounds["since_utc"]))
            until = datetime.fromisoformat(as_text(bounds["until_utc"]))
            if observation.window["kind"] == "lookback":
                seconds = observation.window["seconds"]
                assert isinstance(seconds, int)
                assert (since, until) == (
                    anchors[0] - timedelta(seconds=seconds),
                    anchors[0],
                )
            else:
                assert utc_text(since) == observation.window["since_utc"]
                assert utc_text(until) == observation.window["until_utc"]


def recent_repository(
    root: Path, timestamp: int
) -> tuple[FixtureRepository, JsonObject]:
    repository = FixtureRepository(root=root, evidence=root.parent)
    repository.initialize()
    label = root.name
    repository.write(
        "recent.ts", "function first() { old(); }\n\nfunction last() { old(); }\n"
    )
    for name in ["future", "deleted", "staged", "events"]:
        repository.write(f"{name}.ts", f"function {name}() {{ return 1; }}\n")
    baseline = repository.commit(timestamp - 5000)
    repository.write("events.ts", f'function event_one() {{ return "{label}-two"; }}\n')
    first = repository.commit(timestamp - 20)
    repository.write(
        "events.ts", f'function event_old_parent() {{ return "{label}-three"; }}\n'
    )
    old_parent = repository.commit(timestamp - 2000)
    repository.write(
        "events.ts", f'function event_last() {{ return "{label}-four"; }}\n'
    )
    last = repository.commit(timestamp - 10)
    repository.write(
        "recent.ts",
        f'function first() {{ earlier_edit("{label}"); }}\n\nfunction last() {{ old(); }}\n',
    )
    os.utime(root / "recent.ts", (timestamp - 4000, timestamp - 4000))
    repository.write(
        "recent.ts",
        f'function first() {{ earlier_edit("{label}"); }}\n\nfunction last() {{ recent_edit("{label}"); }}\n',
    )
    os.utime(root / "recent.ts", (timestamp - 5, timestamp - 5))
    repository.write("future.ts", "function future() { return 2; }\n")
    os.utime(root / "future.ts", (timestamp + 3600, timestamp + 3600))
    (root / "deleted.ts").unlink()
    repository.write("staged.ts", "function staged() { return 2; }\n")
    repository.git("add", "--", "staged.ts")
    os.utime(root / "staged.ts", (timestamp - 5, timestamp - 5))
    manifest: JsonObject = {
        "root": str(root),
        "repository_key": label,
        "expected_revision_paths": [
            "events.ts",
            "recent.ts",
            "future.ts",
            "staged.ts",
            "deleted.ts",
        ],
        "unit_texts": [
            f'function first() {{ earlier_edit("{label}"); }}',
            f'function last() {{ recent_edit("{label}"); }}',
        ],
        "baseline": baseline,
        "recent_commit_ids": [first, last],
        "commit_parents": {first: baseline, last: old_parent},
        "old_parent": old_parent,
        "setup_timestamp": timestamp,
        "recent_mtime": timestamp - 5,
        "future_mtime": timestamp + 3600,
        "head": repository.git("rev-parse", "HEAD"),
        "index_tree": repository.git("write-tree"),
        "recent_bytes": (root / "recent.ts").read_text(),
    }
    return repository, manifest


def check_sources(
    report: JsonObject, repository: FixtureRepository, manifest: JsonObject
) -> None:
    key = as_text(manifest["repository_key"])
    for value in array_at(report, "members"):
        member = as_object(value)
        bounds = object_at(member, "bounds")
        paths = [as_object(value) for value in array_at(member, "changed_paths")]
        if bounds["kind"] == "recent_commits":
            expected_commits = {
                as_text(value) for value in array_at(manifest, "recent_commit_ids")
            }
            assert len(paths) == len(expected_commits)
            assert all(value["path"] == "events.ts" for value in paths)
            assert {
                object_at(value, "provenance")["commit_id"] for value in paths
            } == expected_commits
            for changed in paths:
                provenance = object_at(changed, "provenance")
                assert provenance["kind"] == "commit"
                parent = object_at(provenance, "parent")
                assert parent == {
                    "kind": "parent",
                    "commit_id": object_at(manifest, "commit_parents")[
                        as_text(provenance["commit_id"])
                    ],
                    "position": 0,
                }
        elif bounds["kind"] == "revision_diff":
            assert len(paths) == len(array_at(manifest, "expected_revision_paths"))
            assert {value["path"] for value in paths} == set(
                array_at(manifest, "expected_revision_paths")
            )
            assert bounds["base_revision"] == manifest["baseline"]
        for value in array_at(member, "changed_paths"):
            changed = as_object(value)
            path = as_text(changed["path"])
            assert path in [
                "events.ts",
                "recent.ts",
                "future.ts",
                "staged.ts",
                "deleted.ts",
            ]
            provenance = object_at(changed, "provenance")
            for side in ["before", "after"]:
                if changed[side] is None:
                    assert path == "deleted.ts" and side == "after"
                    continue
                source = object_at(changed, side)
                assert (
                    source["kind"] == "repository_file"
                    and source["repository"] == key
                    and source["path"] == path
                )
                revision = object_at(source, "revision")
                assert revision["side"] == side
                if revision["kind"] == "commit":
                    commit = as_text(revision["commit_id"])
                    assert source["content_identity"] == repository.git(
                        "rev-parse", f"{commit}:{path}"
                    )
                    if provenance["kind"] == "commit":
                        expected = (
                            provenance["commit_id"]
                            if side == "after"
                            else object_at(manifest, "commit_parents")[
                                as_text(provenance["commit_id"])
                            ]
                        )
                        assert commit == expected
                    elif bounds["kind"] == "revision_diff":
                        assert (
                            side == "before"
                            and commit == bounds["base_revision"] == provenance["base"]
                        )
                    else:
                        assert (
                            provenance == {"kind": "dirty_layer", "layer": "staged"}
                            and side == "before"
                        )
                        assert commit == repository.git("rev-parse", "HEAD")
                elif revision["kind"] == "index":
                    assert source["content_identity"] == repository.git(
                        "rev-parse", f":{path}"
                    )
                    assert provenance["kind"] == "dirty_layer"
                    assert (provenance["layer"], side) in [
                        ("staged", "after"),
                        ("unstaged", "before"),
                    ]
                else:
                    assert revision["kind"] == "worktree" and side == "after"
                    assert (
                        source["content_identity"]
                        == hashlib.sha256(
                            (repository.root / path).read_bytes()
                        ).hexdigest()
                    )
                    assert provenance["kind"] == "revision_diff" or provenance == {
                        "kind": "dirty_layer",
                        "layer": "unstaged",
                    }


def check_selections(outcome: JsonObject, manifests: list[JsonObject]) -> None:
    selections = [as_object(value) for value in array_at(outcome, "selections")]
    assert len(selections) == len(manifests) == 2
    assert [value["id"] for value in selections] == [
        value["repository_key"] for value in manifests
    ]
    for selected, manifest in zip(selections, manifests):
        key = manifest["repository_key"]
        readiness = object_at(selected, "readiness")
        assert (
            readiness["kind"] == "ready"
            and object_at(readiness, "coverage")["repository"] == key
        )
        units = [as_object(value) for value in array_at(readiness, "content", "units")]
        assert units and all(
            object_at(unit, "unit", "key")["repository"] == key for unit in units
        )
        texts = {value["text"] for value in units}
        assert all(text in texts for text in array_at(manifest, "unit_texts"))


def check_recent_refusal(outcome: JsonObject, case: str) -> None:
    assert outcome["kind"] == "refused"
    diagnostics = [
        as_object(value) for value in array_at(outcome, "diagnostics", "items")
    ]
    assert len(diagnostics) == 1
    expected_code = (
        "invalid_request" if case == "missing-policy" else "invalid_repository_binding"
    )
    expected_location: JsonObject = (
        {"kind": "field", "field_path": ["repositories", "change_scope"]}
        if case == "missing-policy"
        else {"kind": "none"}
    )
    assert (
        diagnostics[0]["code"] == expected_code
        and diagnostics[0]["location"] == expected_location
    )


def recent_journey(context: DriverContext) -> None:
    summaries: list[JsonObject] = []
    cases = [
        (seconds, policy)
        for seconds in [60, 600]
        for policy in ["include_with_diagnostic", "exclude_with_omission"]
    ]
    cases.append((0, "include_with_diagnostic"))
    for position, (seconds, policy) in enumerate(cases):
        root = context.directory / f"case-{position}"
        root.mkdir(parents=True)
        timestamp = int(datetime.now(UTC).timestamp())
        window: JsonObject = (
            {"kind": "lookback", "seconds": seconds}
            if seconds
            else {
                "kind": "absolute",
                "since_utc": utc_text(datetime.fromtimestamp(timestamp - 600, UTC)),
                "until_utc": utc_text(datetime.fromtimestamp(timestamp + 30, UTC)),
            }
        )
        repositories: list[JsonObject] = []
        manifests: list[JsonObject] = []
        for key in ["first", "second"]:
            repository, manifest = recent_repository(root / key, timestamp)
            scope: JsonObject = {
                "kind": "union",
                "members": [
                    {
                        "kind": "recent_commits",
                        "reference": "HEAD",
                        "window": window,
                        "parent_policy": "first_parent",
                    },
                    {
                        "kind": "recent_uncommitted",
                        "layers": ["staged", "unstaged"],
                        "window": window,
                        "unknown_recency": policy,
                    },
                    {
                        "kind": "revision_diff",
                        "base_revision": manifest["baseline"],
                        "head": {"kind": "worktree"},
                    },
                ],
            }
            repositories.append(repository.binding(key, scope))
            manifests.append(manifest)
        request = evidence_plan(
            root,
            {
                key: {"kind": "change_set", "repository": key}
                for key in ["first", "second"]
            },
            [
                {
                    "id": key,
                    "input": key,
                    "select": {
                        "kind": "changed_units",
                        "languages": ["typescript"],
                        "unit_kinds": ["function"],
                        "source_sides": "after_only",
                    },
                }
                for key in ["first", "second"]
            ],
        )
        request["repositories"] = repositories
        request["discovery"] = discovery()
        write_object(
            root / "fixture-manifest.json",
            {"repositories": manifests, "window": window},
        )
        with ControlledServer(root / "http") as server:
            launched = datetime.now(UTC)
            outcome = run_operation(
                context,
                OperationRequest(
                    name=f"window-{position}",
                    operation="prepare_evidence",
                    input_value=request,
                    cwd=root,
                    credential="none",
                ),
            )
            finished = datetime.now(UTC)
            assert outcome["kind"] == "prepared"
            check_windows(
                WindowObservation(
                    outcome=outcome, window=window, launched=launched, finished=finished
                )
            )
            reports = [
                as_object(value) for value in array_at(outcome, "change_selections")
            ]
            assert {report["repository"] for report in reports} == {"first", "second"}
            for report, manifest in zip(
                sorted(reports, key=lambda value: as_text(value["repository"])),
                manifests,
            ):
                key = as_text(report["repository"])
                if seconds:
                    anchor = datetime.fromisoformat(as_text(report["anchor_utc"]))
                    setup = manifest["setup_timestamp"]
                    assert isinstance(setup, int)
                    if anchor.timestamp() - setup > seconds - 20:
                        raise RuntimeError(
                            "US20 setup gap: fixture commits aged outside their intended window; recreate fixtures in a new run, without widening limits"
                        )
                check_sources(
                    report, FixtureRepository(root=root / key, evidence=root), manifest
                )
                members = [as_object(value) for value in array_at(report, "members")]
                assert [member["scope_position"] for member in members] == [0, 1, 2]
                assert set(
                    array_at(object_at(members[0], "bounds"), "commit_ids")
                ) == set(array_at(manifest, "recent_commit_ids"))
                assert object_at(members[0], "recency")["kind"] == "commit_time"
                dirty = members[1]
                assert object_at(dirty, "recency")["kind"] == "approximate_file_time"
                paths = {
                    as_object(value)["path"]
                    for value in array_at(dirty, "changed_paths")
                }
                unknown = object_at(dirty, "recency", "unknown")
                if policy == "include_with_diagnostic":
                    assert paths == {
                        "recent.ts",
                        "future.ts",
                        "deleted.ts",
                        "staged.ts",
                    }
                    assert set(array_at(unknown, "paths")) == {
                        "future.ts",
                        "deleted.ts",
                        "staged.ts",
                    }
                    assert "recency_unknown" in str(dirty["diagnostics"])
                else:
                    assert paths == {"recent.ts"}
                    assert unknown == {"kind": "excluded", "count": 3}
                assert (
                    object_at(members[2], "bounds")["base_revision"]
                    == manifest["baseline"]
                )
            check_selections(outcome, manifests)
            assert server.count == 0
            if position == 0:
                for negative in ["missing-policy", "nested-union"]:
                    invalid = copy.deepcopy(request)
                    binding = as_object(array_at(invalid, "repositories")[0])
                    scope = object_at(binding, "change_scope")
                    if negative == "missing-policy":
                        as_object(array_at(scope, "members")[1]).pop("unknown_recency")
                    else:
                        binding["change_scope"] = {"kind": "union", "members": [scope]}
                    refused = run_operation(
                        context,
                        OperationRequest(
                            name=negative,
                            operation="prepare_evidence",
                            input_value=invalid,
                            cwd=root,
                            credential="none",
                            expected_exit=2,
                        ),
                    )
                    check_recent_refusal(refused, negative)
                    assert server.count == 0
            summaries.append(
                {
                    "window": window,
                    "launched": utc_text(launched),
                    "finished": utc_text(finished),
                    "change_selections": outcome["change_selections"],
                    "http_requests": server.count,
                }
            )
    write_object(context.directory / "recent-readback.json", {"cases": summaries})
