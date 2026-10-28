use schemars::schema_for;
use serde_json::json;
use sunstack_evals::{CaseSetManifest, EvalRunInput, EvalRunOutcome};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &json!({
            "input": schema_for!(EvalRunInput),
            "outcome": schema_for!(EvalRunOutcome),
            "manifest": schema_for!(CaseSetManifest),
        }),
    )?;
    Ok(())
}
