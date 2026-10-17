"""US21: coherent cross-repository local evidence; the genuine slot is spent."""

import copy
import hashlib
from dataclasses import dataclass
from pathlib import Path

from composition_scenarios import native_answer
from git_scenario_fixtures import FixtureRepository, discovery, related_selection
from scenario_fixtures import base_invocation, bindings, noul_rule, ruleset
from scenario_runtime import (
    DriverContext,
    JsonObject,
    OperationRequest,
    array_at,
    as_object,
    as_text,
    complete_report,
    object_at,
    read_object,
    run_operation,
    write_object,
)
from scenario_transport import ControlledServer

LIBRARY_TEXT = "export function target() { return 2; }"
CALLER_TEXT = "import { target as alias } from '@library/lib';\nfunction caller() { return alias(); }"


def connected_fixture(root: Path, endpoint: str, case: str) -> JsonObject:
    root.mkdir(parents=True)
    library = FixtureRepository(root=root / "library", evidence=root)
    library.initialize()
    library.write("lib.ts", "export function target() { return 1; }")
    library.commit()
    library.write("lib.ts", LIBRARY_TEXT)
    caller = FixtureRepository(root=root / "caller", evidence=root)
    caller.initialize()
    caller.write(
        "caller.ts",
        CALLER_TEXT
        + (
            "\nfunction shadow(alias: () => number) { return alias(); }"
            if case == "shadow"
            else ""
        ),
    )
    caller.commit()
    definitions = root / "definitions"
    definitions.mkdir()
    definition = ruleset(["rules.jsonl"])
    definition["inputs"] = {"changes": {"kind": "change_set"}}
    write_object(definitions / "set.json", definition)
    select = related_selection()
    if case == "overflow":
        object_at(select, "chunk_settings")["max_chunk_bytes"] = 1
    write_object(definitions / "rules.jsonl", noul_rule("connected", "changes", select))
    invocation = base_invocation(definitions, endpoint)
    bindings(invocation).clear()
    bindings(invocation)["changes"] = {"kind": "change_set", "repository": "library"}
    invocation["repositories"] = [
        library.binding("library", {"kind": "uncommitted", "layers": ["unstaged"]}),
        caller.binding("caller", {"kind": "full"}),
    ]
    invocation["discovery"] = {"kind": "none"} if case == "disabled" else discovery()
    invocation["package_bindings"] = (
        []
        if case == "missing-mapping"
        else [
            {
                "importing_repository_id": "caller",
                "module_prefix": "@library",
                "target_repository_id": "library",
                "target_path_prefix": "",
                "language": "typescript",
            }
        ]
    )
    object_at(invocation, "attempts")["attempt_timeout_ms"] = 60000
    object_at(invocation, "request_budget")["max_requests"] = 1
    object_at(invocation, "limits")["max_files"] = 100
    return invocation


def unit_identity(unit: JsonObject) -> tuple[str, str, str, str, int, int]:
    start, end = unit["byte_start"], unit["byte_end"]
    assert isinstance(start, int) and isinstance(end, int)
    return (
        as_text(unit["repository"]),
        as_text(unit["path"]),
        as_text(object_at(unit, "revision")["kind"]),
        as_text(object_at(unit, "revision")["side"]),
        start,
        end,
    )


def check_unit_keys(units: list[JsonObject]) -> None:
    assert len(units) == 2
    assert {unit_identity(unit) for unit in units} == {
        ("library", "lib.ts", "worktree", "after", 7, 38),
        ("caller", "caller.ts", "worktree", "after", 48, 85),
    }


def check_group(group: JsonObject, root: Path, shadow: bool = False) -> None:
    excerpts = [as_object(value) for value in array_at(group, "excerpts")]
    assert len(excerpts) == 2
    check_unit_keys([object_at(value, "unit") for value in excerpts])
    check_unit_keys([as_object(value) for value in array_at(group, "units")])
    assert {object_at(value, "unit")["repository"] for value in excerpts} == {
        "library",
        "caller",
    }
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
        excerpt = next(
            value
            for value in excerpts
            if object_at(value, "unit")["repository"] == repository
        )
        assert excerpt["text"] == text and excerpt["membership"] == {
            "kind": "whole_unit"
        }
        span = object_at(excerpt, "span")
        assert (span["byte_start"], span["byte_end"]) == (
            start,
            start + len(text.encode()),
        )
        source = object_at(span, "source")
        assert source["repository"] == repository and source["path"] == path
        assert object_at(source, "revision")["side"] == "after"
        assert (
            source["content_identity"]
            == hashlib.sha256((root / repository / path).read_bytes()).hexdigest()
        )
        assert object_at(excerpt, "context")["enclosing_signature"] == signature
        if repository == "caller":
            imports = array_at(excerpt, "context", "import_context")
            assert len(imports) == 1
            assert (
                as_object(imports[0])["byte_start"],
                as_object(imports[0])["byte_end"],
            ) == (0, 47)
            assert object_at(as_object(imports[0]), "source") == source
    relationships = [as_object(value) for value in array_at(group, "relationships")]
    assert any(
        value["kind"] == "candidate"
        and unit_identity(object_at(value, "source_unit"))
        == ("caller", "caller.ts", "worktree", "after", 48, 85)
        and unit_identity(object_at(value, "target_unit"))
        == ("library", "lib.ts", "worktree", "after", 7, 38)
        for value in relationships
    )
    if shadow:
        assert any(
            value["kind"] == "contradicted"
            and value["contradiction"] == "lexical_shadow"
            for value in relationships
        )
        assert all(
            "function shadow" not in as_text(value["text"]) for value in excerpts
        )


def check_h01(body: JsonObject) -> None:
    for value in object_at(body, "questions").values():
        question = as_object(value)
        assert "instructions" not in question or question["instructions"] is not None
        if question["type"] == "noul" and "criteria" in question:
            criteria = object_at(question, "criteria")
            assert all(
                criteria.get(label, "omitted") is not None
                for label in ["true", "false"]
            )
        if question["type"] == "score":
            assert all(value is not None for value in array_at(question, "criteria"))


def check_connected_report(report: JsonObject) -> None:
    occurrences = [as_object(value) for value in array_at(report, "occurrences")]
    assert len(occurrences) == 1 and occurrences[0]["kind"] == "judged"
    assert occurrences[0]["selected_by"] == ["full_scope"]
    assert object_at(occurrences[0], "key")["rule_id"] == "connected"
    check_unit_keys(
        [
            as_object(unit)
            for unit in array_at(occurrences[0], "judgment", "target", "units")
        ]
    )
    answer = object_at(occurrences[0], "judgment", "verdict", "basis", "answer")
    assert native_answer(report, answer)["type"] == "noul"
    selections = array_at(report, "change_selections")
    assert len(selections) == 1 and as_object(selections[0])["repository"] == "library"
    assert object_at(report, "measurements")["model_requests"] == 1


def source_manifest(root: Path) -> JsonObject:
    files: JsonObject = {}
    for relative in [
        "definitions/set.json",
        "definitions/rules.jsonl",
        "library/lib.ts",
        "caller/caller.ts",
    ]:
        path = root / relative
        files[relative] = {
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "text": path.read_text(),
        }
    repositories: JsonObject = {}
    for name in ["library", "caller"]:
        repository = FixtureRepository(root=root / name, evidence=root)
        repositories[name] = {
            "head": repository.git("rev-parse", "HEAD"),
            "index_tree": repository.git("write-tree"),
            "diff": repository.git("diff", "--binary", "HEAD"),
            "status": repository.git("status", "--short"),
        }
    assert object_at(repositories, "caller")["status"] == ""
    return {"files": files, "repositories": repositories}


@dataclass(frozen=True, kw_only=True)
class FreezeInputs:
    context: DriverContext
    root: Path
    stdin: Path
    http: Path


def freeze_positive(inputs: FreezeInputs) -> None:
    body = read_object(inputs.http)
    check_h01(body)
    frozen = object_at(read_object(inputs.stdin), "input")
    write_object(inputs.context.directory / "local-frozen.json", frozen)
    digest = hashlib.sha256(inputs.http.read_bytes()).hexdigest()
    write_object(
        inputs.context.directory / "h01-readback.json",
        {
            "request_path": str(inputs.http),
            "sha256": digest,
            "questions_checked": len(object_at(body, "questions")),
        },
    )
    write_object(
        inputs.context.directory / "source-manifest.json",
        {
            "root": str(inputs.root),
            "controlled_stdin": str(inputs.stdin),
            "controlled_http": str(inputs.http),
            "product_commit": inputs.context.product_commit,
            "binary_sha256": hashlib.sha256(
                inputs.context.binary.read_bytes()
            ).hexdigest(),
            "sources": source_manifest(inputs.root),
            "http_sha256": digest,
        },
    )


def check_refusal(preparation: JsonObject, outcome: JsonObject, case: str) -> None:
    code = (
        "discovery_not_enabled"
        if case == "disabled"
        else "missing_related_context"
        if case == "missing-mapping"
        else "required_group_incomplete"
    )
    if case == "disabled":
        for value in [preparation, outcome]:
            assert value["kind"] == "invalid"
            assert not any(
                key in value
                for key in [
                    "report",
                    "preparation",
                    "requests",
                    "occurrences",
                    "measurements",
                ]
            )
            assert any(
                as_object(item)["code"] == code
                for item in array_at(value, "diagnostics", "items")
            )
        return
    assert (
        preparation["kind"] == "prepared_with_failures"
        and outcome["kind"] == "incomplete_report"
    )
    prepared = object_at(preparation, "preparation")
    report = object_at(outcome, "report")
    for owner, prepared_side in [(prepared, True), (report, False)]:
        occurrences = [as_object(value) for value in array_at(owner, "occurrences")]
        assert (
            len(occurrences) == 1
            and object_at(occurrences[0], "key")["rule_id"] == "connected"
        )
        status = (
            object_at(occurrences[0], "status") if prepared_side else occurrences[0]
        )
        assert status["kind"] == "selection_failed" and "judgment" not in status
        assert any(
            as_object(item)["code"] == code
            for item in array_at(status, "diagnostics", "items")
        )
        measurements = object_at(owner, "measurements")
        assert measurements["model_requests"] == measurements["inference_attempts"] == 0
    requests = [as_object(value) for value in array_at(report, "requests")]
    assert len(requests) == 1 and requests[0]["questions"] == []
    status = object_at(requests[0], "status")
    assert (
        status["kind"] == "not_sent"
        and object_at(status, "reason")["kind"] == "evidence_refused"
    )
    assert any(
        as_object(item)["code"] == code
        for item in array_at(status, "reason", "diagnostics", "items")
    )


def connected_journey(context: DriverContext) -> None:
    freeze: FreezeInputs | None = None
    summaries: list[JsonObject] = []
    for case in ["positive", "shadow", "missing-mapping", "disabled", "overflow"]:
        root = context.directory / case
        with ControlledServer(context.directory / f"{case}-http") as server:
            invocation = connected_fixture(root, server.endpoint, case)
            positive = case in ["positive", "shadow"]
            exit_code = 0 if positive else 2 if case == "disabled" else 3
            preparation = run_operation(
                context,
                OperationRequest(
                    name=f"{case}-prepare",
                    operation="prepare_rules",
                    input_value=invocation,
                    cwd=root / "definitions",
                    expected_exit=exit_code,
                ),
            )
            assert server.count == 0
            with context.begin(
                OperationRequest(
                    name=f"{case}-evaluate",
                    operation="evaluate_rules",
                    input_value=invocation,
                    cwd=root / "definitions",
                    expected_exit=exit_code,
                )
            ) as process:
                if positive:
                    assert preparation["kind"] == "prepared"
                    planned = array_at(preparation, "preparation", "planned_requests")
                    assert len(planned) == 1
                    selected = as_object(
                        array_at(as_object(planned[0]), "readiness", "evidence")[0]
                    )
                    check_group(
                        object_at(selected, "readiness", "content", "group", "group"),
                        root,
                        case == "shadow",
                    )
                    pending = server.next()
                    assert len(object_at(pending.body, "questions")) == 1
                    attribution = object_at(
                        pending.body, "state", "evidence", "rule:connected/target"
                    )
                    group_outcome = as_object(
                        object_at(pending.body, "state", "contents")[
                            as_text(attribution["content"])
                        ]
                    )
                    assert group_outcome["kind"] == "complete"
                    group = object_at(group_outcome, "group")
                    check_group(group, root, case == "shadow")
                    check_h01(pending.body)
                    pending.answer()
                    report = complete_report(process.finish(0))
                    check_connected_report(report)
                    paths = array_at(
                        as_object(
                            array_at(
                                as_object(array_at(report, "change_selections")[0]),
                                "members",
                            )[0]
                        ),
                        "changed_paths",
                    )
                    after = as_object(paths[0])["after"]
                    library = next(
                        as_object(value)
                        for value in array_at(group, "excerpts")
                        if object_at(as_object(value), "unit")["repository"]
                        == "library"
                    )
                    assert object_at(library, "span")["source"] == after
                    assert server.count == 1
                    if case == "positive":
                        freeze = FreezeInputs(
                            context=context,
                            root=root,
                            stdin=process.directory / "stdin.json",
                            http=context.directory / "positive-http/http-01.raw.json",
                        )
                else:
                    outcome = process.finish(exit_code)
                    check_refusal(preparation, outcome, case)
                    assert server.count == 0
            summaries.append(
                {"case": case, "exit": exit_code, "http_requests": server.count}
            )
    assert freeze is not None
    freeze_positive(freeze)
    write_object(context.directory / "connected-readback.json", {"cases": summaries})


def connected_checks(evidence: Path) -> None:
    body: JsonObject = {
        "questions": {
            "one": {
                "type": "noul",
                "instructions": {"question": "judge connected"},
                "criteria": {"true": "yes", "false": "no"},
            }
        }
    }
    check_h01(body)
    malformed = copy.deepcopy(body)
    object_at(malformed, "questions", "one")["instructions"] = None
    null_refused = False
    try:
        check_h01(malformed)
    except AssertionError:
        null_refused = True
    else:
        raise AssertionError("H01 body refusal was lost")
    write_object(evidence / "h01-checks.json", {"null_body_refused": null_refused})
