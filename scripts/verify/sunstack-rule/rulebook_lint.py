# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic==2.14.1", "pydantic_core==2.50.1"]
# ///
"""Observe rulebook lint through a declared binary; --prepare never runs a product.

From the monorepo root: uv run --script <this-file> --prepare --evidence <new-dir>.
To observe omit --prepare and add --binary <absolute> --source-root <checkout>
--commit <full-head>. Drives --evidence for every kind, the worked examples with
owned loopback replies, temporary Git scope changes, and skip/malformed cases.
No real key, keychain access, hosted endpoint, product patch or test is used.
"""

import argparse
import math
import subprocess
import sys
from pathlib import Path

from rulebook_evidence_checks import listing_pieces, location, require_piece_contract
from rulebook_fixture_support import (
    CONFIG,
    ORACLE,
    RULEBOOK,
    RULES,
    SKILL,
    JsonObject,
    array_value,
    baseline_git,
    configure,
    expected_steps,
    object_value,
    prepare_fixture,
    read_object,
    require,
    select_worked_skill,
    write_json,
)
from rulebook_process import RulebookProcess, declared_target, response_body


def scripted_reply(
    body: JsonObject, *, total: float = 0.58, confidence: float = 0.86
) -> JsonObject:
    reply = response_body(body, 0.91, "outcome")
    questions = object_value(body["questions"])
    if "kind" in questions:
        answers = object_value(reply["answers"])
        where = object_value(object_value(body["state"])["where"])
        selected = where["file"] == SKILL and where["first_line"] == 49
        coverage = (
            (0.30 if total == 0.58 else (total - 0.4) / 0.6) if selected else 0.95
        )
        object_value(answers["covers"])["noul"] = coverage
        if selected:
            answer = object_value(answers["kind"])
            answer["confidence"] = confidence
            answer["probabilities"] = {
                "outcome": confidence,
                "action": (1 - confidence) * 0.7,
                "other": (1 - confidence) * 0.3,
            }
    return reply


def status(report: JsonObject) -> str:
    return str(object_value(report["status"])["kind"])


def findings(report: JsonObject) -> list[JsonObject]:
    return [object_value(item) for item in array_value(report["findings"])]


def finding_line(finding: JsonObject) -> int:
    return int(str(finding["line"]))


def require_worked_results(report: JsonObject) -> None:
    rows = findings(report)
    require(
        any(
            item["marker"] == "K002"
            and finding_line(item) == 3
            and item["severity"] == "error"
            for item in rows
        ),
        "K002 error at description line 3 absent",
    )
    selected = [
        item for item in rows if item["marker"] == "K001" and finding_line(item) == 59
    ]
    require(
        len(selected) == 1
        and selected[0]["severity"] == "warn"
        and selected[0]["field"] == "completion",
        "K001 warning location/field differs",
    )
    require(
        math.isclose(
            float(str(object_value(selected[0]["judgment"])["value"])),
            0.58,
            abs_tol=1e-12,
        ),
        "Combined result was rounded or weighted incorrectly",
    )
    require(not any(item["marker"] == "K003" for item in rows), "K003 0.91 should pass")


def require_uncertain(report: JsonObject, step: str) -> None:
    selected = [
        object_value(item)
        for item in array_value(report["uncertain"])
        if object_value(item)["marker"] == "K001"
        and object_value(object_value(item)["where"])["first_line"] == 49
    ]
    require(len(selected) == 1, "K001 uncertain was lost")
    item = selected[0]
    require(item["step"] == step, "Wrong uncertain step")
    require(
        set(object_value(item["answers"])) == {"kind", "covers", "checkable"},
        "Uncertain answers incomplete",
    )
    require(bool(object_value(item["state"])), "Uncertain sent state missing")


def configure_endpoint(root: Path, endpoint: str) -> None:
    configure(
        root,
        lambda config: object_value(config["decision_model"]).update(endpoint=endpoint),
    )


def lint_args(*extra: str) -> list[str]:
    return ["lint", "--config", CONFIG, "--format", "json", *extra]


def observe(process: RulebookProcess, fixtures: Path) -> None:
    root = fixtures / "all-kinds"
    configure_endpoint(root, process.server.endpoint)
    out, _, calls = process.run(
        root,
        "evidence-all",
        ["lint", "--config", CONFIG, "--evidence", "--all"],
        expected_exit=0,
    )
    require(not calls, "--evidence dispatched a request")
    pieces = listing_pieces(out)
    require_piece_contract(pieces, root)
    for kind in object_value(
        object_value(read_object(root / CONFIG)["evidence"])["kinds"]
    ):
        out, _, calls = process.run(
            root,
            f"evidence-{kind}",
            ["lint", "--config", CONFIG, "--evidence", kind, "--all"],
            expected_exit=0,
        )
        require(
            not calls
            and listing_pieces(out)
            == [piece for piece in pieces if piece["kind"] == kind],
            f"Kind filter differs: {kind}",
        )
    worked = fixtures / "worked"
    configure_endpoint(worked, process.server.endpoint)
    process.reply = scripted_reply
    baseline, requests = process.report(
        worked, "worked", lint_args("--all"), expected_exit=1, credential=True
    )
    require(status(baseline) == "findings", "Wrong full-fixture status")
    require_worked_results(baseline)
    require(bool(requests), "Worked examples never asked")
    repeated_report, repeated_requests = process.report(
        worked, "worked-repeat", lint_args("--all"), expected_exit=1, credential=True
    )
    require(
        [item.raw for item in requests] == [item.raw for item in repeated_requests],
        "Worked-example request order or bytes changed",
    )
    for key in ("findings", "uncertain", "counts", "exclusions", "incomplete"):
        require(
            baseline[key] == repeated_report[key],
            f"Worked-example result order/content changed: {key}",
        )
    rules = read_object(worked / RULES)
    for pending in requests:
        questions = object_value(pending.body["questions"])
        marker = "K001" if "kind" in questions else "K003"
        require(
            questions == object_value(object_value(rules[marker])["questions"]),
            "Questions changed on the wire",
        )
        state = object_value(pending.body["state"])
        require(
            "text" not in state and "standard" in state and "where" in state,
            "Request includes raw text or misses standard/location",
        )
    for name, total, confidence, expected_step in [
        ("quality", 0.65, 0.86, "quality"),
        ("choice", 0.58, 0.55, "kind"),
    ]:
        process.reply = lambda body, total=total, confidence=confidence: scripted_reply(
            body, total=total, confidence=confidence
        )
        isolated = fixtures / "uncertain-only"
        configure_endpoint(isolated, process.server.endpoint)
        report, _ = process.report(
            isolated,
            f"uncertain-{name}",
            lint_args("--all"),
            expected_exit=5,
            credential=True,
        )
        require(status(report) == "uncertain", "Uncertain-only status differs")
        require_uncertain(report, expected_step)
    process.reply = lambda body: scripted_reply(body, total=0.65)
    report, _ = process.report(
        worked,
        "uncertain-plus-error",
        lint_args("--all"),
        expected_exit=1,
        credential=True,
    )
    require(status(report) == "findings", "Uncertain incorrectly outranks error")
    require_uncertain(report, "quality")
    process.reply = scripted_reply
    missing = fixtures / "missing-completion"
    configure_endpoint(missing, process.server.endpoint)
    report, missing_requests = process.report(
        missing,
        "missing-completion",
        lint_args("--all"),
        expected_exit=0,
        credential=True,
    )
    rows = findings(report)
    require(
        not missing_requests
        and len(rows) == 1
        and rows[0]["marker"] == "K001"
        and finding_line(rows[0]) == 49
        and rows[0]["judgment"] is None,
        "Missing completion must settle mechanically at line 49 without a request",
    )
    skipped = fixtures / "skip"
    configure_endpoint(skipped, process.server.endpoint)
    report, _ = process.report(
        skipped, "skip", lint_args("--all"), expected_exit=3, credential=True
    )
    require(
        status(report) == "incomplete" and report["findings"] == baseline["findings"],
        "Bad entry changed peer findings",
    )
    require(
        bool(array_value(report["skipped_rules"])),
        "Invalid entry not reported as skipped",
    )
    require(
        any(
            object_value(object_value(item)["reason"])["kind"] == "coverage_partial"
            for item in array_value(report["incomplete"])
        ),
        "Coverage gap missing",
    )
    malformed = fixtures / "malformed"
    report, calls = process.report(
        malformed, "malformed", lint_args("--all"), expected_exit=2
    )
    require(
        status(report) == "invalid"
        and not calls
        and object_value(report["usage"])["checked_files_acquired"] == 0,
        "Malformed config acquired checked content",
    )
    observe_changes(process, fixtures)


def observe_changes(process: RulebookProcess, fixtures: Path) -> None:
    root = fixtures / "changes"
    configure_endpoint(root, process.server.endpoint)
    baseline_git(root)
    original = (root / SKILL).read_text()
    lines = original.splitlines(keepends=True)
    lines[54] = lines[54].rstrip("\n") + " Verify this changed obligation.\n"
    (root / SKILL).write_text("".join(lines))
    out, _, _ = process.run(
        root,
        "changed-step",
        ["lint", "--config", CONFIG, "--evidence", "step", "--since", "HEAD"],
        expected_exit=0,
    )
    picked = listing_pieces(out)
    require(
        len(picked) == 1 and location(picked[0])["first_line"] == 49,
        "Changed lines picked another step",
    )
    report, requests = process.report(
        root,
        "changed-step-run",
        lint_args("--since", "HEAD"),
        expected_exit=0,
        credential=True,
    )
    require(
        object_value(report["exclusions"])["outside_change_scope"] == 5,
        "Five unchanged steps were not excluded",
    )
    require(len(requests) == 1, "Changed scope asked about other steps")
    require(
        object_value(report["usage"])["checked_files_acquired"] == 1,
        "Changed file must count as one content acquisition, including captured after text",
    )
    repeat, repeated = process.report(
        root,
        "changed-step-repeat",
        lint_args("--since", "HEAD"),
        expected_exit=0,
        credential=True,
    )
    require(
        [request.raw for request in repeated] == [request.raw for request in requests],
        "Request order or bytes were nondeterministic",
    )
    for key in ("findings", "uncertain", "counts", "exclusions", "incomplete"):
        require(report[key] == repeat[key], f"Result order/content changed: {key}")
    root = fixtures / "changed-load"
    configure_endpoint(root, process.server.endpoint)
    baseline_git(root)
    loaded = root / "skills/verification-design/references/choosing-the-observation.md"
    loaded.write_text(loaded.read_text() + "\nChanged loaded reference.\n")
    out, _, _ = process.run(
        root,
        "changed-load",
        ["lint", "--config", CONFIG, "--evidence", "load", "--since", "HEAD"],
        expected_exit=0,
    )
    require(
        any(
            location(piece)["first_line"] == 51 and location(piece)["file"] == SKILL
            for piece in listing_pieces(out)
        ),
        "Changed target did not pick its load",
    )
    root = fixtures / "rule-change"
    configure_endpoint(root, process.server.endpoint)
    baseline_git(root)
    (root / RULES).write_text((root / RULES).read_text() + "\n")
    report, _ = process.report(
        root,
        "rule-change-widens",
        lint_args("--since", "HEAD"),
        expected_exit=1,
        credential=True,
    )
    require(bool(object_value(report["scope"])["widened"]), "Rule edit did not widen")
    untracked = fixtures / "untracked"
    configure_endpoint(untracked, process.server.endpoint)
    baseline_git(untracked)
    path = untracked / "skills/new-skill/SKILL.md"
    path.parent.mkdir()
    path.write_text(original)
    out, _, _ = process.run(
        untracked,
        "untracked-whole",
        ["lint", "--config", CONFIG, "--evidence", "step"],
        expected_exit=0,
    )
    rows = listing_pieces(out)
    require(
        len(rows) == 6
        and all(
            location(piece)["file"] == "skills/new-skill/SKILL.md" for piece in rows
        ),
        "Untracked skill not wholly picked",
    )


def prepare(destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=False)
    for name in (
        "all-kinds",
        "worked",
        "uncertain-only",
        "skip",
        "malformed",
        "missing-completion",
        "changes",
        "changed-load",
        "rule-change",
        "untracked",
    ):
        markers = (
            ("K001",)
            if name in ("uncertain-only", "changes", "untracked", "missing-completion")
            else ()
        )
        root = prepare_fixture(destination / name, markers=markers)
        if name in (
            "worked",
            "uncertain-only",
            "skip",
            "changes",
            "rule-change",
            "missing-completion",
        ):
            select_worked_skill(root)
        if name == "changes":

            def step_only(config: JsonObject) -> None:
                evidence = object_value(config["evidence"])
                evidence["kinds"] = {"step": object_value(evidence["kinds"])["step"]}

            configure(root, step_only)
        if name == "missing-completion":
            lines = (root / SKILL).read_text().splitlines(keepends=True)
            lines[58] = "\n"
            (root / SKILL).write_text("".join(lines))
            write_json(
                root / "evidence/rules/selected-step.yml",
                {
                    "id": "selected-step",
                    "language": "Markdown",
                    "files": ["**/SKILL.md"],
                    "rule": {
                        "all": [
                            {"matches": "step"},
                            {"regex": r"^### 3\. Choose each observation"},
                        ]
                    },
                },
            )

            def selected_step(config: JsonObject) -> None:
                evidence = object_value(config["evidence"])
                step = object_value(object_value(evidence["kinds"])["step"])
                step["rule"] = "selected-step"
                evidence["kinds"] = {"step": step}

            configure(root, selected_step)
        if name == "untracked":
            configure(
                root,
                lambda config: config.update(check={"files": ["skills/**/SKILL.md"]}),
            )
        if name == "skip":
            rules = read_object(root / RULES)
            rules["K004"] = {
                "name": "invalid_peer",
                "evidence": "absent_kind",
                "decision": {
                    "start": "done",
                    "steps": {
                        "done": {
                            "mechanical_check": {
                                "tool": "contains",
                                "field": "body",
                                "text": "x",
                            },
                            "yes": "pass",
                            "no": "pass",
                        }
                    },
                },
            }
            write_json(root / RULES, rules)
            (root / RULEBOOK).write_text(
                (root / RULEBOOK).read_text() + "\nAnother standard. <!-- K004 -->\n"
            )
        if name == "malformed":
            (root / CONFIG).write_text('{"rulebook": [\n')
    # Independently confirm the selected S0 source bytes before any product run.
    root = destination / "all-kinds"
    for filename in (
        "verification-design-step-49.json",
        "verification-design-load-51.json",
        "practices-retrospective-step-19.json",
    ):
        record = read_object(root / "expected" / filename)
        source = (root / str(record["file"])).read_bytes()
        require(
            source[int(str(record["start"])) : int(str(record["end"]))].decode()
            == record["text"],
            "S0 expected source differs",
        )
    require(len(expected_steps(root)) == 6, "Fixture no longer has six steps")


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
        prepare(evidence / "fixtures")
        if args.prepare:
            write_json(
                evidence / "prepared.json",
                {"product_observed": False, "oracle": ORACLE, "fixture_cases": 10},
            )
            print("Prepared rulebook journey fixtures; product unobserved")
            return
        if target is None:
            raise ValueError("Declared target missing")
        with RulebookProcess(target=target, evidence=evidence / "run") as process:
            observe(process, evidence / "fixtures")
        write_json(
            evidence / "pass.json",
            {
                "commit": target.commit,
                "binary_sha256": target.digest,
                "oracle": ORACLE,
                "required_joins": [
                    "U1/U2/U3 native evidence tests",
                    "U5 edge/walk tables",
                    "U6 request cap cases",
                    "U7/U9 recording gate tests",
                    "U11 source review",
                    "hosted inference remains unobserved",
                ],
            },
        )
        print("Rulebook binary journeys passed; inspect required native/hosted joins")
    except (
        ValueError,
        TypeError,
        OSError,
        KeyError,
        subprocess.SubprocessError,
    ) as error:
        parser.exit(2, f"Rulebook lint observation failed: {error}\n")


if __name__ == "__main__":
    main()
