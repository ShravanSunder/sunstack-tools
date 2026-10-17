"""Synthetic invocation fixtures adapted from the public native/process consumers."""

from dataclasses import dataclass
from pathlib import Path

from scenario_runtime import JsonObject, as_object, write_object


@dataclass(kw_only=True)
class RuleFixture:
    root: Path
    input_value: JsonObject


def declaration(input_name: str, selector_id: str = "target") -> JsonObject:
    return {
        "id": selector_id,
        "input": input_name,
        "select": {"kind": "paragraph", "target_id": "U001"},
    }


def noul_rule(
    rule_id: str = "one",
    input_name: str = "subject",
    selection: JsonObject | None = None,
) -> JsonObject:
    target = declaration(input_name)
    if selection is not None:
        target["select"] = selection
    return {
        "rule_id": rule_id,
        "evidence": [target],
        "target": "target",
        "applicability": [],
        "questions": {},
        "verdict": {
            "kind": "noul",
            "question": {"type": "noul", "instructions": f"judge {rule_id}"},
            "threshold": 0.8,
        },
    }


def decision(input_name: str) -> JsonObject:
    return {
        "kind": "decision",
        "question": {"type": "noul", "instructions": f"gate {input_name}"},
        "evidence": [declaration(input_name, "context")],
        "threshold": 0.6,
    }


def conversation_selection(
    turn_kind: str = "all", assistant: str = "all", tools: bool = True
) -> JsonObject:
    return {
        "kind": "conversation",
        "turns": {"kind": turn_kind},
        "user_messages": "included",
        "assistant_messages": assistant,
        "tool_use": {
            "kind": "included",
            "tools": {"kind": "all"},
            "results": "included",
        }
        if tools
        else {"kind": "excluded"},
        "hook_injected": "excluded",
    }


def limits() -> JsonObject:
    return {
        "max_files": 32,
        "max_file_bytes": 262144,
        "max_total_bytes": 1048576,
        "max_matches": 100,
        "max_transcript_bytes": 262144,
        "include_ignored": False,
    }


def document_binding(path: str) -> JsonObject:
    return {"kind": "document", "repository": "repo", "path": path}


def repository(root: Path) -> JsonObject:
    return {
        "key": "repo",
        "root": str(root),
        "input_paths": [{"kind": "glob", "pattern": "**"}],
        "change_scope": {"kind": "full"},
    }


def base_invocation(root: Path, endpoint: str) -> JsonObject:
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
                                "subject": document_binding("source.md"),
                                "other": document_binding("other.md"),
                            }
                        }
                    ],
                }
            ],
        },
        "repositories": [repository(root)],
        "payload_schemas": [],
        "origin": {"kind": "direct"},
        "cursors": [],
        "execution": {"speculation": "off", "max_concurrent_requests": 4},
        "limits": limits(),
        "backend": {
            "kind": "jev_openrouter",
            "endpoint": endpoint,
            "model": "jev-latest",
        },
        "attempts": {
            "attempt_timeout_ms": 5000,
            "retry_count": 0,
            "initial_backoff_ms": 1,
            "max_backoff_ms": 1,
            "max_response_bytes": 1048576,
        },
        "request_budget": {
            "max_input_tokens": 64000,
            "max_request_bytes": 524288,
            "max_questions": 32,
            "max_requests": 20,
            "safety_margin_tokens": 32,
            "accounting_policy": "allow_estimated",
        },
    }


def ruleset(
    rules: list[str], applicability: list[JsonObject] | None = None
) -> JsonObject:
    return {
        "schema_version": 1,
        "name": "usage scenario",
        "description": "caller composition",
        "inputs": {"subject": {"kind": "document"}, "other": {"kind": "document"}},
        "applicability": list(applicability or []),
        "common_ruleset_instructions": "SCENARIO_COMMON",
        "rule_paths": rules,
    }


def simple_fixture(
    root: Path, endpoint: str, applicability: list[JsonObject] | None = None
) -> RuleFixture:
    root.mkdir(parents=True, exist_ok=False)
    (root / "source.md").write_text("U001: ORIGINAL_SUBJECT\n")
    (root / "other.md").write_text("U001: ORIGINAL_OTHER\n")
    write_object(root / "set.json", ruleset(["rules.jsonl"], applicability))
    write_object(root / "rules.jsonl", noul_rule())
    return RuleFixture(root=root, input_value=base_invocation(root, endpoint))


def evidence_plan(
    root: Path, sources: JsonObject, declarations: list[JsonObject]
) -> JsonObject:
    return {
        "repositories": [repository(root)],
        "sources": sources,
        "payload_schemas": [],
        "cursors": [],
        "native_queries": [],
        "package_bindings": [],
        "discovery": {"kind": "none"},
        "declarations": list(declarations),
        "limits": limits(),
    }


def bindings(input_value: JsonObject) -> JsonObject:
    definitions = as_object(input_value["definitions"])
    groups = definitions.get("binding_groups")
    if groups is None:
        rule_sets = definitions["rule_sets"]
        assert isinstance(rule_sets, list)
        groups = as_object(rule_sets[0])["binding_groups"]
    assert isinstance(groups, list)
    return as_object(as_object(groups[0])["bindings"])
