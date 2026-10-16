use crate::{request_state::*, *};
use std::collections::BTreeMap;
use sunstack_decisions::{
    AdmittedBackendRequest, DecisionBackend, JudgmentQuestion, QuestionName,
    RequestBudgetAssessment, RequestBudgetSettings, StructuredJudgmentRequest,
    assess_request_budget,
};
#[derive(Debug, Clone)]
pub(crate) struct RuleRequestBlock {
    pub key: RuleOccurrenceKey,
    pub rule: std::sync::Arc<RuleDefinition>,
    pub evidence: PreparedRuleEvidence,
    pub common: sunstack_decisions::StructuredEntry,
}
pub(crate) struct EncodedRuleRequest {
    pub request: StructuredJudgmentRequest,
    pub admitted: AdmittedBackendRequest,
    pub bindings: Vec<QuestionBinding>,
}
pub(crate) fn encode_rules(
    blocks: &[RuleRequestBlock],
    backend: &dyn DecisionBackend,
    budget: &RequestBudgetSettings,
) -> Result<EncodedRuleRequest, DiagnosticList> {
    let first = encode_once(blocks, backend, budget, false);
    if first.is_err()
        && blocks.iter().any(|block| {
            block.evidence.selections.iter().any(|selection| {
                selection.declaration.requirement
                    == sunstack_evidence::EvidenceRequirement::Optional
            })
        })
    {
        encode_once(blocks, backend, budget, true)
    } else {
        first
    }
}
fn encode_once(
    blocks: &[RuleRequestBlock],
    backend: &dyn DecisionBackend,
    budget: &RequestBudgetSettings,
    omit_optional: bool,
) -> Result<EncodedRuleRequest, DiagnosticList> {
    let mut questions = BTreeMap::new();
    let mut bindings = Vec::new();
    for block in blocks {
        let owner = format!("rule:{}", block.rule.rule_id.as_str());
        let reads = block
            .evidence
            .selections
            .iter()
            .map(|selection| selection.label.clone())
            .collect::<Vec<_>>();
        let mut add = |question: &JudgmentQuestion,
                       question_owner: QuestionOwner|
         -> Result<(), DiagnosticList> {
            let request_name =
                QuestionName::new(format!("q{:06}", questions.len().saturating_add(1))).map_err(
                    |_| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError),
                )?;
            questions.insert(
                request_name.clone(),
                wrap_question(question, &owner, reads.clone(), Some(&block.common))?,
            );
            bindings.push(QuestionBinding {
                request_name,
                owner: question_owner,
            });
            Ok(())
        };
        if let VerdictDeclaration::Noul { question, .. } = &block.rule.verdict {
            add(
                question,
                QuestionOwner::Verdict {
                    rule_id: block.rule.rule_id.clone(),
                },
            )?;
        }
        for (name, question) in &block.rule.questions {
            add(
                question,
                QuestionOwner::Informational {
                    rule_id: block.rule.rule_id.clone(),
                    name: name.clone(),
                },
            )?;
        }
    }
    let state = build_request_state(
        blocks
            .iter()
            .flat_map(|block| block.evidence.selections.clone()),
        omit_optional,
    )?;
    let request = StructuredJudgmentRequest { questions, state };
    let encoded = backend.encode_request(&request).map_err(|diagnostic| {
        DiagnosticList::from(sunstack_decisions::DiagnosticList::from(diagnostic))
    })?;
    let accounting = backend.account_request(&encoded);
    match assess_request_budget(encoded, budget, accounting) {
        RequestBudgetAssessment::WithinConfiguredBudget { admitted_request } => {
            Ok(EncodedRuleRequest {
                request,
                admitted: admitted_request,
                bindings,
            })
        }
        RequestBudgetAssessment::Exceeded { diagnostics }
        | RequestBudgetAssessment::Unverifiable { diagnostics } => Err(diagnostics.into()),
    }
}
