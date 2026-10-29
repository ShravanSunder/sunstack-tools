# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic==2.14.1", "pydantic_core==2.50.1"]
# ///
"""Current S1 rulebook lint journey, implemented in the repo's scripts home.

Use --prepare --evidence <new-directory> for offline fixture preparation.
For a declared local binary omit --prepare and pass --binary --source-root
and full --commit. The config is rulebook/check/evidence/decision_model.
Historical RuleSet fixture helpers remain native/historical proof sources;
they do not drive this current journey. See references/lint-runner.md for joins.
"""

import runpy
import sys
from pathlib import Path

if __name__ == "__main__":
    script = (
        Path(__file__).resolve().parents[4]
        / "scripts/verify/sunstack-rule/lint_runner_rulebook.py"
    )
    sys.path.insert(0, str(script.parent))
    runpy.run_path(str(script), run_name="__main__")
