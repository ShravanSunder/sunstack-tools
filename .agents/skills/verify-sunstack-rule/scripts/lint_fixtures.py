# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic==2.14.1", "pydantic_core==2.50.1"]
# ///
"""S1: build independent lint fixture repositories, or drive a declared sunstack.

From the monorepo root: uv run --script <this-file> --prepare --evidence <new-dir>
builds fixtures without a product. Omit --prepare and add --binary, --source-root,
and full --commit for a commissioned observation. No genuine provider request
is made: HTTP stays on an owned loopback server with synthetic authorization.
Success receipts cover only executed cases; reference lists native-proof gaps.
Container mode adds --container-image (adopted Alpine digest) and
--container-control-host (declared route to the owned loopback server).
No-call children have network none; only init can write its fixture.
"""

import argparse
import subprocess
import sys
from pathlib import Path

from lint_observations import (
    check_report,
    require_diagnostic,
    require_format,
    require_invalid_report,
    require_rules,
)
from runner_containers import ContainerRuntime
from runner_contracts import JSON_OBJECT, FixtureCase, as_object, write_json
from runner_fixture_cases import build_fixtures
from runner_fixture_setup import write_fixture
from runner_process import RunnerTarget, binary_digest, run_case
from scenario_transport import ControlledServer


def observe_init(
    target: RunnerTarget,
    root: Path,
    evidence: Path,
    case: FixtureCase,
    server: ControlledServer,
) -> None:
    run_case(target, root, evidence / "init-create", case, server)
    skeleton = JSON_OBJECT.validate_json((root / "sunstack.lint.json").read_bytes())
    if (
        set(skeleton) != {"$schema", "rule_sets"}
        or skeleton["rule_sets"] != {}
        or not skeleton["$schema"]
    ):
        raise ValueError("--init did not produce C4 skeleton")
    initial = (root / "sunstack.lint.json").read_bytes()
    refusal = case.model_copy(update={"name": "init-refuse-overwrite", "exit_code": 2})
    run_case(target, root, evidence / "init-refuse-overwrite", refusal, server)
    if (root / "sunstack.lint.json").read_bytes() != initial:
        raise ValueError("--init overwrote existing config")
    invalid = case.model_copy(
        update={
            "name": "init-empty-check",
            "arguments": ["lint", "--check"],
            "mode": "check",
            "exit_code": 2,
        }
    )
    run_case(target, root, evidence / "init-empty-check", invalid, server)
    valid_fixture = root / "valid"
    write_fixture(valid_fixture)
    skeleton["rule_sets"] = {
        "docs": {"path": "valid/rules/set.json", "files": ["valid/docs/*"]}
    }
    write_json(root / "sunstack.lint.json", skeleton)
    valid = invalid.model_copy(update={"name": "init-populated-check", "exit_code": 0})
    run_case(target, root, evidence / "init-populated-check", valid, server)


def observe(target: RunnerTarget, evidence: Path, selected: list[str]) -> None:
    with ControlledServer(evidence / "http") as server:
        fixtures = evidence / "fixtures"
        endpoint = server.endpoint
        if target.container:
            endpoint = target.container.control_endpoint(endpoint)
            target.container.confirm_control(endpoint, evidence)
        manifest = build_fixtures(fixtures, endpoint)
        names = {case.name for case in manifest.cases}
        if set(selected) - names:
            raise ValueError(f"Unknown cases: {set(selected) - names}")
        cases = [
            case for case in manifest.cases if not selected or case.name in selected
        ]
        if not cases:
            raise ValueError("No cases selected")
        executed: list[str] = []
        for case in cases:
            root = fixtures / case.name
            comparison_root = target.comparison_root(root)
            if case.mode == "init":
                observe_init(target, root, evidence / "processes", case, server)
                executed.append(case.name)
                continue
            initial_requests = server.count
            _, stdout, stderr = run_case(
                target, root, evidence / "processes" / case.name, case, server
            )
            require_diagnostic(stdout.decode(), case, stderr.decode(), comparison_root)
            if case.mode == "check" and case.exit_code == 2:
                structured_case = case.model_copy(
                    update={
                        "name": f"{case.name}-json-diagnostic",
                        "arguments": [
                            word for word in case.arguments if word != "--check"
                        ]
                        + ["--all", "--format", "json"],
                        "mode": "report",
                    }
                )
                _, structured, _ = run_case(
                    target,
                    root,
                    evidence / "processes" / structured_case.name,
                    structured_case,
                    server,
                )
                require_invalid_report(
                    JSON_OBJECT.validate_json(structured), case, comparison_root
                )
            if case.mode == "report":
                report = JSON_OBJECT.validate_json(stdout)
                states = [
                    as_object(pending.body["state"])
                    for pending in server.received[initial_requests:]
                ]
                check_report(report, case, comparison_root, states)
                if case.status == "nothing_selected":
                    text_case = case.model_copy(
                        update={
                            "arguments": [
                                word
                                for word in case.arguments
                                if word not in ("--format", "json")
                            ]
                        }
                    )
                    _, text, errors = run_case(
                        target,
                        root,
                        evidence / "processes" / f"{case.name}-text",
                        text_case,
                        server,
                    )
                    rendered = text.decode() + errors.decode()
                    if "No checks selected" not in rendered or "Passed" in rendered:
                        raise ValueError("Nothing selected was presented as Passed")
                if case.name in ("mechanical-error", "mixed-formats"):
                    for format_name in ("default", "github", "sarif"):
                        format_case = case.model_copy(
                            update={
                                "arguments": ["lint", "--all", "--format", format_name]
                            }
                        )
                        _, text, _ = run_case(
                            target,
                            root,
                            evidence / "processes" / f"{case.name}-{format_name}",
                            format_case,
                            server,
                        )
                        require_format(
                            text.decode(), report, format_name, comparison_root
                        )
            elif case.mode == "rules":
                require_rules(stdout.decode(), case, comparison_root)
            executed.append(case.name)
        target.confirm()
        write_json(
            evidence / "s1-observation.json",
            {
                "commit": target.commit,
                "binary_sha256": target.digest,
                "cases": executed,
                "case_count": len(executed),
                "http_requests": server.count,
                "genuine_requests": 0,
                "gaps": manifest.gaps,
            },
        )
    # The server and all child processes have settled before this success file.
    write_json(
        evidence / "s1-pass.json", {"observed_cases": executed, "gaps": manifest.gaps}
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", required=True, type=Path)
    parser.add_argument("--prepare", action="store_true")
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--source-root", type=Path, default=Path.cwd())
    parser.add_argument("--commit")
    parser.add_argument(
        "--container-image",
        help="Adopted pinned Alpine digest; needs --container-control-host",
    )
    parser.add_argument(
        "--container-control-host",
        help="Declared route from containers to this run's loopback server",
    )
    parser.add_argument("--case", action="append", default=[])
    arguments = parser.parse_args()
    if bool(arguments.container_image) != bool(arguments.container_control_host):
        parser.error(
            "Container mode requires both --container-image and --container-control-host"
        )
    if sys.flags.optimize:
        parser.error("Unset PYTHONOPTIMIZE and omit -O/-OO")
    if not arguments.prepare and (arguments.binary is None or arguments.commit is None):
        parser.error(
            "Observation needs --binary and full --commit; --prepare only builds fixtures"
        )
    try:
        target = (
            None
            if arguments.prepare
            else RunnerTarget(
                binary=arguments.binary.resolve(),
                source=arguments.source_root.resolve(),
                commit=arguments.commit,
                digest=binary_digest(arguments.binary),
                container=ContainerRuntime(
                    image=arguments.container_image,
                    control_host=arguments.container_control_host,
                )
                if arguments.container_image
                else None,
            )
        )
        if target:
            target.confirm()
            if target.container:
                target.container.confirm_image()
        evidence = arguments.evidence.resolve()
        evidence.mkdir(parents=True, exist_ok=False)
        if arguments.prepare:
            manifest = build_fixtures(evidence / "fixtures")
            write_json(
                evidence / "fixtures-prepared.json",
                {
                    "cases": len(manifest.cases),
                    "oracle": manifest.oracle,
                    "product_observed": False,
                    "gaps": manifest.gaps,
                },
            )
            print(f"Prepared {len(manifest.cases)} fixtures; no product observed")
        else:
            if target is None:
                raise ValueError("Declared target missing")
            observe(target, evidence, arguments.case)
            print(
                f"S1 selected cases passed; see {evidence}; native/hosted gaps remain named"
            )
    except (
        ValueError,
        TypeError,
        OSError,
        TimeoutError,
        subprocess.SubprocessError,
    ) as error:
        parser.exit(2, f"S1 setup/observation failed: {error}\n")


if __name__ == "__main__":
    main()
