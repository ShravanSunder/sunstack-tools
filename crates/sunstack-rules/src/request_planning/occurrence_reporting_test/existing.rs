#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::super::*;
use crate::rule_definitions::invocation_admission::{AdmittedGroup, LoadedRuleSet};
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc};
fn reduction_plan() -> ExecutionPlan {
    let rule:RuleDefinition=serde_json::from_value(json!({"rule_id":"one","evidence":[{"id":"target","input":"subject","select":{"kind":"paragraph","target_id":"U001"}}],"target":"target","applicability":[],"questions":{},"verdict":{"kind":"noul","question":{"type":"noul","instructions":"judge"},"threshold":0.8}})).unwrap();
    let definition:RuleSetDefinition=serde_json::from_value(json!({"schema_version":1,"name":"reduction","description":"settled typed states","inputs":{"subject":{"kind":"document"}},"applicability":[{"kind":"decision","question":{"type":"noul","instructions":"gate"},"evidence":[{"id":"gate","input":"subject","select":{"kind":"paragraph","target_id":"U001"}}],"threshold":0.6}],"common_ruleset_instructions":null,"rule_paths":["rules.jsonl"]})).unwrap();
    let rule = Arc::new(rule);
    let set = Arc::new(LoadedRuleSet {
        path: "set.json".into(),
        definition,
        rules: vec![LocatedRuleDefinition {
            source: "rules.jsonl".into(),
            line: 1,
            definition: rule,
        }],
        skipped: vec![],
    });
    let group = AdmittedGroup {
        set,
        position: 0,
        binding: GroupBinding::Bound,
        sources: BTreeMap::new(),
    };
    let mut plan = definition_topology(vec![group], None);
    plan.occurrences[0].scope = ScopeEvaluation::pending();
    plan
}
#[test]
fn settled_gate_reduction_wins_over_comparison_refusal_in_both_application_orders() {
    // Pure reduction coverage: real Git/refusal/HTTP wiring is covered by change_selection.rs.
    // These orders deliberately do not claim two public-runtime completion orders.
    for passing in [false, true] {
        for comparison_first in [false, true] {
            let plan = reduction_plan();
            let mut state = ExecutionState::new(&plan);
            let reference = AnswerReference {
                request: RequestId(0),
                question: sunstack_decisions::QuestionName::new("q000001").unwrap(),
            };
            let gate = EntrySettlement::Decision {
                threshold: 0.6,
                result: if passing {
                    DecisionSettlement::Passed { reference }
                } else {
                    DecisionSettlement::DidNotPass { reference }
                },
            };
            let refused = OwnerPreparation::Refused {
                selections: vec![],
                diagnostics: sunstack_evidence::DiagnosticList::one(
                    sunstack_evidence::EvidenceCode::DuplicateTarget,
                )
                .into(),
            };
            let gate_index = plan.groups[0].entries[0];
            if comparison_first {
                state
                    .preparation
                    .insert(UnitKey::Rule(OccurrenceIndex(0)), refused);
                state.entries[gate_index.0] = gate;
            } else {
                state.entries[gate_index.0] = gate;
                state
                    .preparation
                    .insert(UnitKey::Rule(OccurrenceIndex(0)), refused);
            }
            let result = occurrence_result(OccurrenceIndex(0), &plan, &state, &BTreeMap::new());
            if passing {
                assert!(
                    matches!(result,RuleOccurrenceResult::SelectionFailed{diagnostics,..} if diagnostics.items.iter().any(|item|matches!(item,ReportDiagnostic::Evidence(diagnostic) if diagnostic.code==sunstack_evidence::EvidenceCode::DuplicateTarget)))
                );
            } else {
                assert!(matches!(result, RuleOccurrenceResult::NotApplicable { .. }));
            }
        }
    }
}
