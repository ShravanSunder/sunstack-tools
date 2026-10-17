"""Permanent journey-readback controls, invoked only by the selector's --check mode."""

import copy
import hashlib
import typing as t
from datetime import UTC, datetime, timedelta
from functools import partial
from pathlib import Path

from connected_scenarios import (
    check_group,
    check_refusal,
    check_unit_keys,
    connected_fixture,
)
from dependency_scenarios import SelectionExpectation, check_selection_pair
from git_scenario_fixtures import FixtureRepository
from recent_scenarios import (
    WindowObservation,
    check_recent_refusal,
    check_selections,
    check_sources,
    check_windows,
    recent_repository,
    utc_text,
)
from scenario_runtime import (
    JsonObject,
    array_at,
    as_object,
    as_text,
    object_at,
    write_object,
)
from usage_scenarios import require_same_sources, skill_snapshot


def assertion_rejects(operation: t.Callable[[], object]) -> bool:
    try:
        operation()
    except AssertionError:
        return True
    return False


def correction_checks(evidence: Path) -> None:
    repository = FixtureRepository(
        root=evidence / "dirty-source-control", evidence=evidence
    )
    repository.initialize()
    repository.write("skill/driver.py", "baseline\n")
    repository.commit()
    repository.write("skill/driver.py", "already dirty\n")
    before_status = repository.git("status", "--short")
    before = skill_snapshot(repository.root / "skill")
    repository.write("skill/driver.py", "different dirty contents\n")
    after_status = repository.git("status", "--short")
    after = skill_snapshot(repository.root / "skill")
    assert before_status == after_status and before != after
    assert assertion_rejects(partial(require_same_sources, before, after))
    units: list[JsonObject] = [
        {
            "repository": key,
            "path": path,
            "revision": {"kind": "worktree", "side": "after"},
            "byte_start": start,
            "byte_end": end,
        }
        for key, path, start, end in [
            ("library", "lib.ts", 7, 38),
            ("caller", "caller.ts", 48, 85),
        ]
    ]
    check_unit_keys(units)
    unit_refusals = 0
    for field in [
        "repository",
        "path",
        "revision.kind",
        "revision.side",
        "byte_start",
        "byte_end",
    ]:
        malformed = copy.deepcopy(units)
        if field.startswith("revision."):
            object_at(malformed[1], "revision")[field.split(".")[1]] = "wrong"
        else:
            malformed[1][field] = 999 if field.startswith("byte_") else "wrong"
        unit_refusals += int(assertion_rejects(partial(check_unit_keys, malformed)))
    assert unit_refusals == 6
    diagnostic: JsonObject = {
        "items": [{"code": "missing_related_context"}],
        "omitted_count": 0,
    }
    prepared: JsonObject = {
        "kind": "prepared_with_failures",
        "preparation": {
            "occurrences": [
                {
                    "key": {"rule_id": "connected"},
                    "status": {"kind": "selection_failed", "diagnostics": diagnostic},
                }
            ],
            "measurements": {"model_requests": 0, "inference_attempts": 0},
        },
    }
    report: JsonObject = {
        "kind": "incomplete_report",
        "report": {
            "occurrences": [
                {
                    "kind": "selection_failed",
                    "key": {"rule_id": "connected"},
                    "diagnostics": diagnostic,
                }
            ],
            "requests": [
                {
                    "questions": [],
                    "status": {
                        "kind": "not_sent",
                        "reason": {
                            "kind": "evidence_refused",
                            "diagnostics": diagnostic,
                        },
                    },
                }
            ],
            "measurements": {"model_requests": 0, "inference_attempts": 0},
        },
    }
    check_refusal(prepared, report, "missing-mapping")
    refused = 0
    for mode in ["judged", "sent", "model_requests", "inference_attempts"]:
        malformed = copy.deepcopy(report)
        if mode == "judged":
            as_object(array_at(malformed, "report", "occurrences")[0])["kind"] = (
                "judged"
            )
        elif mode == "sent":
            object_at(
                as_object(array_at(malformed, "report", "requests")[0]), "status"
            )["kind"] = "sent"
        else:
            object_at(malformed, "report", "measurements")[mode] = 1
        refused += int(
            assertion_rejects(
                partial(check_refusal, prepared, malformed, "missing-mapping")
            )
        )
    assert refused == 4
    manifests: list[JsonObject] = [
        {"repository_key": key, "unit_texts": [key + "-literal"]}
        for key in ["first", "second"]
    ]
    selections: JsonObject = {
        "selections": [
            {
                "id": key,
                "readiness": {
                    "kind": "ready",
                    "coverage": {"repository": key},
                    "content": {
                        "units": [
                            {
                                "unit": {"key": {"repository": key}},
                                "text": key + "-literal",
                            }
                        ]
                    },
                },
            }
            for key in ["first", "second"]
        ]
    }
    check_selections(selections, manifests)
    malformed = copy.deepcopy(selections)
    array_at(malformed, "selections").clear()
    assert assertion_rejects(partial(check_selections, malformed, manifests))
    array_at(selections, "selections").reverse()
    assert assertion_rejects(partial(check_selections, selections, manifests))
    diagnostic_refusals = 0
    for mode in ["empty", "code", "location"]:
        invalid: JsonObject = {
            "kind": "refused",
            "diagnostics": {
                "items": [
                    {
                        "code": "invalid_request",
                        "location": {
                            "kind": "field",
                            "field_path": ["repositories", "change_scope"],
                        },
                    }
                ]
            },
        }
        items = array_at(invalid, "diagnostics", "items")
        if mode == "empty":
            items.clear()
        elif mode == "code":
            as_object(items[0])["code"] = "wrong"
        else:
            as_object(items[0])["location"] = {"kind": "none"}
        diagnostic_refusals += int(
            assertion_rejects(partial(check_recent_refusal, invalid, "missing-policy"))
        )
    assert diagnostic_refusals == 3
    write_object(
        evidence / "correction-checks.json",
        {
            "dirty_status_unchanged": before_status == after_status,
            "dirty_content_change_refused": before != after,
            "unit_key_mutations_refused": unit_refusals,
            "malformed_refusals_rejected": refused,
            "empty_and_swapped_selections_refused": 2,
            "diagnostic_mutations_refused": diagnostic_refusals,
        },
    )


def readback_checks(evidence: Path) -> None:
    root = evidence / "group-readback-control"
    connected_fixture(root, "http://127.0.0.1:1/systemone", "positive")
    excerpts: list[JsonObject] = []
    for repository, path, start, text, signature in [
        (
            "library",
            "lib.ts",
            7,
            "function target() { return 2; }",
            "function target()",
        ),
        (
            "caller",
            "caller.ts",
            48,
            "function caller() { return alias(); }",
            "function caller()",
        ),
    ]:
        source: JsonObject = {
            "repository": repository,
            "path": path,
            "revision": {"kind": "worktree", "side": "after"},
            "content_identity": hashlib.sha256(
                (root / repository / path).read_bytes()
            ).hexdigest(),
        }
        span: JsonObject = {
            "source": source,
            "byte_start": start,
            "byte_end": start + len(text),
        }
        excerpts.append(
            {
                "unit": {
                    "repository": repository,
                    "path": path,
                    "revision": {"kind": "worktree", "side": "after"},
                    "byte_start": start,
                    "byte_end": start + len(text),
                },
                "text": text,
                "span": span,
                "membership": {"kind": "whole_unit"},
                "context": {
                    "enclosing_signature": signature,
                    "import_context": [
                        {"byte_start": 0, "byte_end": 47, "source": source}
                    ]
                    if repository == "caller"
                    else [],
                },
            }
        )
    group: JsonObject = {
        "excerpts": excerpts,
        "units": [object_at(value, "unit") for value in excerpts],
        "relationships": [
            {
                "kind": "candidate",
                "source_unit": object_at(excerpts[1], "unit"),
                "target_unit": object_at(excerpts[0], "unit"),
            }
        ],
    }
    check_group(group, root)
    refused = 0
    for mode in ["missing-caller", "wrong-caller-text", "wrong-owner"]:
        invalid = copy.deepcopy(group)
        selected = invalid["excerpts"]
        assert isinstance(selected, list)
        if mode == "missing-caller":
            selected.pop()
        else:
            caller = selected[1]
            assert isinstance(caller, dict)
            if mode == "wrong-caller-text":
                caller["text"] = "function caller() { return other(); }"
            else:
                object_at(caller, "unit")["repository"] = "wrong-root"
        refused += int(assertion_rejects(partial(check_group, invalid, root)))
    preparation: JsonObject = {
        "preparation": {
            "occurrences": [
                {
                    "status": {
                        "kind": "planned",
                        "selection": {
                            "kind": "selected",
                            "selected_by": ["target_changed"],
                        },
                    }
                },
                {
                    "status": {
                        "kind": "outside_change_scope",
                        "reason": "no_declared_change",
                    }
                },
            ]
        }
    }
    report: JsonObject = {
        "occurrences": [
            {
                "kind": "judged",
                "key": {"rule_id": "changed"},
                "selected_by": ["target_changed"],
                "judgment": {
                    "verdict": {
                        "basis": {"answer": {"request_index": 0, "question_name": "q1"}}
                    }
                },
            },
            {
                "kind": "outside_change_scope",
                "key": {"rule_id": "neighbor"},
                "reason": "no_declared_change",
            },
        ],
        "requests": [
            {"status": {"outcome": {"answers": {"q1": {"type": "noul", "noul": 0.9}}}}}
        ],
    }
    expected = SelectionExpectation(
        kinds=("judged", "outside_change_scope"), reasons=(["target_changed"], [])
    )
    check_selection_pair(preparation, report, expected)
    for mode in ["empty-occurrences", "wrong-reason", "false-outside"]:
        invalid = copy.deepcopy(report)
        rows = invalid["occurrences"]
        assert isinstance(rows, list)
        if mode == "empty-occurrences":
            rows.clear()
        else:
            row = rows[0]
            assert isinstance(row, dict)
            if mode == "wrong-reason":
                row["selected_by"] = ["full_scope"]
            else:
                row["kind"] = "outside_change_scope"
        refused += int(
            assertion_rejects(
                partial(check_selection_pair, preparation, invalid, expected)
            )
        )
    assert refused == 6
    write_object(
        evidence / "readback-checks.json",
        {"malformed_group_and_selection_observations_refused": refused},
    )


def source_checks(evidence: Path) -> None:
    repository, manifest = recent_repository(
        evidence / "source-identity-control", int(datetime.now(UTC).timestamp())
    )
    baseline = as_text(manifest["baseline"])
    before: JsonObject = {
        "kind": "repository_file",
        "repository": "first",
        "path": "recent.ts",
        "revision": {"kind": "commit", "commit_id": baseline, "side": "before"},
        "content_identity": repository.git("rev-parse", f"{baseline}:recent.ts"),
    }
    after: JsonObject = {
        "kind": "repository_file",
        "repository": "first",
        "path": "recent.ts",
        "revision": {"kind": "worktree", "snapshot": "synthetic", "side": "after"},
        "content_identity": hashlib.sha256(
            (repository.root / "recent.ts").read_bytes()
        ).hexdigest(),
    }
    report: JsonObject = {
        "members": [
            {
                "bounds": {"kind": "revision_diff", "base_revision": baseline},
                "changed_paths": [
                    {
                        "path": "recent.ts",
                        "before": before,
                        "after": after,
                        "provenance": {"kind": "revision_diff", "base": baseline},
                    }
                ],
            }
        ]
    }
    manifest["repository_key"] = "first"
    manifest["expected_revision_paths"] = ["recent.ts"]
    check_sources(report, repository, manifest)
    refused = 0
    for mode in [
        "repository",
        "worktree-digest",
        "git-blob",
        "source-side",
        "empty-paths",
    ]:
        invalid = copy.deepcopy(report)
        changed = as_object(
            array_at(as_object(array_at(invalid, "members")[0]), "changed_paths")[0]
        )
        if mode == "empty-paths":
            array_at(
                as_object(array_at(invalid, "members")[0]), "changed_paths"
            ).clear()
        elif mode == "repository":
            object_at(changed, "after")["repository"] = "wrong-root"
        elif mode == "worktree-digest":
            object_at(changed, "after")["content_identity"] = "0" * 64
        elif mode == "git-blob":
            object_at(changed, "before")["content_identity"] = "0" * 40
        else:
            object_at(changed, "after", "revision")["side"] = "before"
        try:
            check_sources(invalid, repository, manifest)
        except AssertionError:
            refused += 1
    assert refused == 5
    commit_paths: list[JsonObject] = []
    for value in array_at(manifest, "recent_commit_ids"):
        commit = as_text(value)
        parent = as_text(object_at(manifest, "commit_parents")[commit])
        refs: JsonObject = {}
        for side, revision in [("before", parent), ("after", commit)]:
            refs[side] = {
                "kind": "repository_file",
                "repository": "first",
                "path": "events.ts",
                "revision": {"kind": "commit", "commit_id": revision, "side": side},
                "content_identity": repository.git(
                    "rev-parse", f"{revision}:events.ts"
                ),
            }
        commit_paths.append(
            {
                "path": "events.ts",
                "before": refs["before"],
                "after": refs["after"],
                "provenance": {
                    "kind": "commit",
                    "commit_id": commit,
                    "parent": {"kind": "parent", "commit_id": parent, "position": 0},
                },
            }
        )
    committed: JsonObject = {
        "members": [
            {"bounds": {"kind": "recent_commits"}, "changed_paths": commit_paths}
        ]
    }
    check_sources(committed, repository, manifest)
    wrong = copy.deepcopy(committed)
    entry = as_object(
        array_at(as_object(array_at(wrong, "members")[0]), "changed_paths")[0]
    )
    other = as_text(manifest["old_parent"])
    parent = repository.git("rev-parse", other + "^")
    entry["provenance"] = {
        "kind": "commit",
        "commit_id": other,
        "parent": {"kind": "parent", "commit_id": parent, "position": 0},
    }
    for side, revision in [("before", parent), ("after", other)]:
        object_at(entry, side, "revision")["commit_id"] = revision
        object_at(entry, side)["content_identity"] = repository.git(
            "rev-parse", f"{revision}:events.ts"
        )
    wrong_commit_refused = assertion_rejects(
        partial(check_sources, wrong, repository, manifest)
    )
    assert wrong_commit_refused
    write_object(
        evidence / "source-identity-checks.json",
        {
            "malformed_source_identities_refused": refused,
            "consistent_wrong_commit_refused": wrong_commit_refused,
        },
    )


def window_checks(evidence: Path) -> None:
    anchor = datetime(2026, 1, 1, 0, 1, tzinfo=UTC)
    bounds: JsonObject = {
        "kind": "recent_uncommitted",
        "since_utc": utc_text(anchor - timedelta(seconds=60)),
        "until_utc": utc_text(anchor),
    }
    outcome: JsonObject = {
        "change_selections": [
            {
                "repository": key,
                "anchor_utc": utc_text(anchor),
                "members": [{"bounds": bounds}],
            }
            for key in ["first", "second"]
        ]
    }
    observation = WindowObservation(
        outcome=outcome,
        window={"kind": "lookback", "seconds": 60},
        launched=anchor - timedelta(seconds=1),
        finished=anchor + timedelta(seconds=1),
    )
    check_windows(observation)
    refused = 0
    for mode in ["anchor-mismatch", "duration", "outside-bracket"]:
        malformed = copy.deepcopy(outcome)
        if mode == "anchor-mismatch":
            as_object(array_at(malformed, "change_selections")[1])["anchor_utc"] = (
                utc_text(anchor + timedelta(seconds=1))
            )
        elif mode == "duration":
            object_at(
                as_object(
                    array_at(
                        as_object(array_at(malformed, "change_selections")[0]),
                        "members",
                    )[0]
                ),
                "bounds",
            )["since_utc"] = utc_text(anchor - timedelta(seconds=59))
        else:
            for value in array_at(malformed, "change_selections"):
                as_object(value)["anchor_utc"] = utc_text(
                    anchor + timedelta(seconds=30)
                )
        try:
            check_windows(
                WindowObservation(
                    outcome=malformed,
                    window=observation.window,
                    launched=observation.launched,
                    finished=observation.finished,
                )
            )
        except AssertionError:
            refused += 1
    assert refused == 3
    write_object(
        evidence / "window-checks.json",
        {
            "delayed_launch_control": utc_text(anchor),
            "malformed_observations_refused": refused,
        },
    )
