use schemars::schema_for;
use serde_json::json;
use sunstack_rules::{CandidateRunInput, CandidateRunOutcome, RuleDefinition};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &json!({
            "input": schema_for!(CandidateRunInput),
            "outcome": schema_for!(CandidateRunOutcome),
            "rule": schema_for!(RuleDefinition),
        }),
    )?;
    Ok(())
}
