# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic==2.14.1", "pydantic_core==2.50.1"]
# ///
"""Select only the usage journeys the caller commissioned; inventory is in the reference."""

import argparse
import hashlib
import os
import subprocess
import sys
from pathlib import Path

from composition_scenarios import (
    frontier_journey,
    grouping_journey,
    preparation_journey,
    routing_journey,
)
from cursor_scenarios import cursor_journey
from handoff_scenario import handoff_journey, hosted_handoff
from hook_scenarios import origin_journey, payload_journey
from scenario_runtime import (
    INDEX_VARIABLES,
    REPOSITORY_ROOT,
    SKILL_ROOT,
    DriverContext,
    checked_source,
    write_object,
)


def main() -> None:
    if sys.flags.optimize:
        raise SystemExit(
            "Verification requires assertions; unset PYTHONOPTIMIZE and omit -O/-OO"
        )
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--journey", action="append", default=[])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--hosted", action="store_true")
    arguments = parser.parse_args()
    assert not any(os.environ.get(name) for name in INDEX_VARIABLES), (
        "Unset index/config overrides before starting uv"
    )
    evidence = Path(arguments.evidence).resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    environment["UV_CACHE_DIR"] = str(evidence / "uv-cache")
    environment["RUFF_CACHE_DIR"] = str(evidence / "ruff-cache")
    environment["PYTHONDONTWRITEBYTECODE"] = "1"
    if arguments.check:
        assert not arguments.journey and not arguments.hosted
        subprocess.run(
            [
                "uv",
                "run",
                "--no-config",
                "--no-env-file",
                "--no-python-downloads",
                "--default-index",
                "https://pypi.org/simple",
                "--locked",
                "--script",
                str(SKILL_ROOT / "scripts/scenario_checks.py"),
                "--evidence",
                str(evidence),
            ],
            cwd=REPOSITORY_ROOT,
            env=environment,
            check=True,
        )
        return
    assert arguments.journey and arguments.binary, (
        "Select requested IDs and the declared binary"
    )
    selected: list[str] = arguments.journey
    assert len(set(selected)) == len(selected), (
        "Do not repeat a journey in one invocation"
    )
    dispatch = {
        "US15": cursor_journey,
        "US06": preparation_journey,
        "US09": grouping_journey,
        "US03": origin_journey,
        "US04": payload_journey,
        "US11": frontier_journey,
        "US12": routing_journey,
        "US02": handoff_journey,
    }
    assert all(journey in dispatch for journey in selected), (
        "Unknown journey: use the reference inventory"
    )
    if arguments.hosted:
        assert selected == ["US02"], (
            "Only the separately authorized US02 slot is available"
        )
    product, head = checked_source()
    skill_source_status = subprocess.run(
        ["git", "status", "--short", "--", ".agents/skills/verify-sunstack-rule"],
        cwd=REPOSITORY_ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    binary = Path(arguments.binary).resolve(strict=True)
    for journey in selected:
        context = DriverContext(
            binary=binary,
            evidence=evidence,
            journey=journey,
            product_commit=product,
            source_head=head,
        )
        receipt = context.directory / (
            "hosted-pass.json" if arguments.hosted else "local-pass.json"
        )
        assert not receipt.exists(), (
            "Use a new authorized evidence directory for another local run"
        )
        (hosted_handoff if arguments.hosted else dispatch[journey])(context)
        assert checked_source() == (product, head), "Target changed during observation"
        write_object(
            receipt,
            {
                "journey": journey,
                "provider": "hosted" if arguments.hosted else "controlled-loopback",
                "product_commit": product,
                "source_head": head,
                "skill_source_status": skill_source_status,
                "binary": str(binary),
                "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                "processes": [
                    record.model_dump(mode="json") for record in context.records
                ],
            },
        )
        print(
            f"{journey}: passed; {len(context.records)} processes; evidence {context.directory}"
        )


if __name__ == "__main__":
    main()
