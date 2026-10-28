use schemars::schema_for;
use serde_json::json;
use sunstack_rules::{
    CandidateRunInput, CandidateRunOutcome, CheckDefinitionsInput, EvidenceBinding, RuleDefinition,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &json!({
            "input": schema_for!(CandidateRunInput),
            "check_definitions": schema_for!(CheckDefinitionsInput),
            "outcome": schema_for!(CandidateRunOutcome),
            "rule": schema_for!(RuleDefinition),
            "evidence_binding": schema_for!(EvidenceBinding),
        }),
    )?;
    Ok(())
}
