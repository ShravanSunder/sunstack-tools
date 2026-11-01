use crate::{
    rule_definitions::definition_contracts::*, rule_definitions::rule_diagnostics::*,
    rule_definitions::strict_decoding::decode_definition,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use sunstack_decisions::{
    JudgmentQuestion, QuestionName, SharedEvidenceState, StructuredJudgmentRequest,
};
use sunstack_evidence::{EvidenceDeclaration, EvidenceSelection, SourceName};
fn refusal(code: RuleDiagnosticCode) -> DiagnosticList {
    DiagnosticList::rule(code)
}
pub fn decode_rule_set(bytes: &[u8], file: &Path) -> Result<RuleSetDefinition, DiagnosticList> {
    let definition: RuleSetDefinition = decode_definition(bytes, file, 1)?;
    validate_rule_set(&definition).map_err(|mut diagnostics| {
        locate(&mut diagnostics, file, 1);
        diagnostics
    })?;
    Ok(definition)
}
pub fn decode_rule_definition(
    bytes: &[u8],
    file: &Path,
    line: usize,
) -> Result<RuleDefinition, DiagnosticList> {
    decode_definition(bytes, file, line)
}
pub fn decode_router(bytes: &[u8], file: &Path) -> Result<RouterDefinition, DiagnosticList> {
    let router: RouterDefinition = decode_definition(bytes, file, 1)?;
    validate_router_definition(&router).map_err(|mut diagnostics| {
        locate(&mut diagnostics, file, 1);
        diagnostics
    })?;
    Ok(router)
}
fn validate_router_definition(router: &RouterDefinition) -> Result<(), DiagnosticList> {
    if router.schema_version != 1 {
        return Err(refusal(RuleDiagnosticCode::UnsupportedSchemaVersion));
    }
    if router.name.is_empty()
        || router.description.is_empty()
        || router.inputs.is_empty()
        || router.routes.is_empty()
        || router.routes.values().any(Vec::is_empty)
        || !threshold_valid(router.confidence_floor)
    {
        return Err(refusal(RuleDiagnosticCode::InvalidRoute));
    }
    validate_question(&router.question)?;
    let JudgmentQuestion::Choice { criteria, .. } = &router.question else {
        return Err(refusal(RuleDiagnosticCode::InvalidRoute));
    };
    if router
        .routes
        .keys()
        .any(|label| !criteria.contains_key(label))
    {
        return Err(refusal(RuleDiagnosticCode::InvalidRoute));
    }
    validate_evidence(&router.evidence, &router.inputs)?;
    Ok(())
}
pub(crate) fn locate(diagnostics: &mut DiagnosticList, file: &Path, line: usize) {
    locate_at(diagnostics, file, line, 1);
}
pub(crate) fn locate_at(diagnostics: &mut DiagnosticList, file: &Path, line: usize, column: usize) {
    for diagnostic in &mut diagnostics.items {
        match diagnostic {
            ReportDiagnostic::Rule(diagnostic) => {
                if matches!(diagnostic.location, RuleDiagnosticLocation::None) {
                    diagnostic.location = RuleDiagnosticLocation::Definition {
                        file: file.into(),
                        line,
                        column,
                        field_path: vec![],
                    };
                }
            }
            ReportDiagnostic::Decision(diagnostic) => {
                let field_path = match &diagnostic.location {
                    sunstack_decisions::DecisionDiagnosticLocation::None => vec![],
                    sunstack_decisions::DecisionDiagnosticLocation::Field { field_path } => {
                        field_path
                            .iter()
                            .filter(|field| field.as_str() != "validation")
                            .take(32)
                            .cloned()
                            .collect()
                    }
                    _ => continue,
                };
                diagnostic.location = sunstack_decisions::DecisionDiagnosticLocation::Definition {
                    file: file.into(),
                    line,
                    column,
                    field_path,
                };
            }
            ReportDiagnostic::Evidence(diagnostic) => {
                let field_path = match &diagnostic.location {
                    sunstack_evidence::DiagnosticLocation::None => vec![],
                    sunstack_evidence::DiagnosticLocation::Field { field_path } => {
                        field_path.iter().take(32).cloned().collect()
                    }
                    _ => continue,
                };
                diagnostic.location = sunstack_evidence::DiagnosticLocation::Definition {
                    file: file.into(),
                    line,
                    column,
                    field_path,
                };
            }
        }
    }
}
pub fn validate_rule_set(definition: &RuleSetDefinition) -> Result<(), DiagnosticList> {
    if definition.schema_version != 1 {
        return Err(refusal(RuleDiagnosticCode::UnsupportedSchemaVersion));
    }
    if definition.name.is_empty()
        || definition.description.is_empty()
        || definition.inputs.is_empty()
        || definition.rule_paths.is_empty()
        || definition
            .rule_paths
            .iter()
            .any(|path| path.as_os_str().is_empty())
    {
        return Err(refusal(RuleDiagnosticCode::InvalidConfiguration));
    }
    validate_entries(&definition.applicability, &definition.inputs)
}
pub fn validate_rule_definition(
    rule: &RuleDefinition,
    inputs: &BTreeMap<SourceName, InputDeclaration>,
) -> Result<(), DiagnosticList> {
    validate_evidence(&rule.evidence, inputs)?;
    let target = rule
        .evidence
        .iter()
        .find(|declaration| declaration.id == rule.target)
        .ok_or_else(|| refusal(RuleDiagnosticCode::InvalidTarget))?;
    if let VerdictDeclaration::Mechanical { explanation, check } = &rule.verdict {
        if explanation.is_empty() || !rule.questions.is_empty() {
            return Err(refusal(RuleDiagnosticCode::InvalidMechanicalCheck));
        }
        if !matches!(
            target.select,
            EvidenceSelection::MarkdownUnits(_) | EvidenceSelection::FrontmatterField { .. }
        ) || matches!(
            (&target.select, check),
            (
                EvidenceSelection::FrontmatterField { .. },
                crate::MechanicalCheck::AstGrep { .. }
            )
        ) || matches!(
            check,
            crate::MechanicalCheck::Expect {
                expect: crate::ExpectPredicate::Present { .. }
            }
        ) && !matches!(target.select, EvidenceSelection::FrontmatterField { .. })
        {
            return Err(refusal(RuleDiagnosticCode::InvalidTarget));
        }
        validate_entries(&rule.applicability, inputs)?;
        return check.validate();
    }
    if !matches!(
        target.select,
        EvidenceSelection::Paragraph { .. }
            | EvidenceSelection::MarkdownUnits(_)
            | EvidenceSelection::Conversation { .. }
            | EvidenceSelection::WholePayload {}
            | EvidenceSelection::PayloadField { .. }
            | EvidenceSelection::ChangedUnits { .. }
            | EvidenceSelection::RelatedUnits { .. }
    ) {
        return Err(refusal(RuleDiagnosticCode::InvalidTarget));
    }
    validate_entries(&rule.applicability, inputs)?;
    for question in rule.questions.values() {
        validate_question(question)?
    }
    let threshold = rule
        .verdict
        .threshold()
        .ok_or_else(|| refusal(RuleDiagnosticCode::InvalidThreshold))?;
    if !threshold_valid(threshold) {
        return Err(refusal(RuleDiagnosticCode::InvalidThreshold));
    }
    if rule.verdict.band().is_some_and(|band| {
        !band.is_finite() || band < 0.0 || band >= threshold || rule.severity != Severity::Error
    }) {
        return Err(refusal(RuleDiagnosticCode::InvalidBand));
    }
    match &rule.verdict {
        VerdictDeclaration::Mechanical { .. } => {
            return Err(refusal(RuleDiagnosticCode::InvalidMechanicalCheck));
        }
        VerdictDeclaration::Noul { question, .. } => {
            if !matches!(question, JudgmentQuestion::Noul { .. }) {
                return Err(refusal(RuleDiagnosticCode::InvalidComposite));
            }
            validate_question(question)?;
        }
        VerdictDeclaration::Composite { dimensions, .. } => {
            let mut seen = BTreeSet::new();
            let mut total = 0.0;
            if dimensions.is_empty() {
                return Err(refusal(RuleDiagnosticCode::InvalidComposite));
            }
            for dimension in dimensions {
                if !seen.insert(&dimension.question)
                    || !dimension.weight.is_finite()
                    || dimension.weight <= 0.0
                    || dimension.weight > 1.0
                {
                    return Err(refusal(RuleDiagnosticCode::InvalidComposite));
                }
                total += dimension.weight;
                match rule.questions.get(&dimension.question) {
                    Some(JudgmentQuestion::Noul { .. }) => {}
                    Some(JudgmentQuestion::Score { criteria, .. }) if criteria.len() > 1 => {}
                    _ => return Err(refusal(RuleDiagnosticCode::InvalidComposite)),
                }
            }
            if (total - 1.0).abs() > f64::EPSILON * (dimensions.len() as f64).max(1.0) * 4.0 {
                return Err(refusal(RuleDiagnosticCode::InvalidComposite));
            }
        }
    }
    Ok(())
}
pub(crate) fn validate_question(question: &JudgmentQuestion) -> Result<(), DiagnosticList> {
    let name = QuestionName::new("validation")
        .map_err(|_| refusal(RuleDiagnosticCode::InvalidConfiguration))?;
    sunstack_decisions::judgment_processing::judgment_validation::validate_request(
        &StructuredJudgmentRequest {
            questions: BTreeMap::from([(name, question.clone())]),
            state: SharedEvidenceState::Text("admission".into()),
        },
    )
    .map_err(|diagnostic| sunstack_decisions::DiagnosticList::from(diagnostic).into())
}
pub(crate) fn threshold_valid(value: f64) -> bool {
    value.is_finite() && value > 0.0 && value <= 1.0
}
pub(crate) fn validate_evidence(
    declarations: &[EvidenceDeclaration],
    inputs: &BTreeMap<SourceName, InputDeclaration>,
) -> Result<(), DiagnosticList> {
    if declarations.is_empty() {
        return Err(refusal(RuleDiagnosticCode::InvalidConfiguration));
    }
    let mut ids = BTreeSet::new();
    for declaration in declarations {
        if !ids.insert(&declaration.id) {
            return Err(refusal(RuleDiagnosticCode::DuplicateField));
        }
        let input = inputs
            .get(&declaration.input)
            .ok_or_else(|| refusal(RuleDiagnosticCode::UndeclaredInput))?;
        let compatible = matches!(
            (input, &declaration.select),
            (
                InputDeclaration::Document {},
                EvidenceSelection::Paragraph { .. }
                    | EvidenceSelection::MarkdownUnits(_)
                    | EvidenceSelection::FrontmatterField { .. }
                    | EvidenceSelection::WholeDocument {}
                    | EvidenceSelection::LiteralText { .. }
                    | EvidenceSelection::RegexText { .. }
                    | EvidenceSelection::AstPattern { .. }
            ) | (
                InputDeclaration::Conversation {},
                EvidenceSelection::Conversation { .. }
            ) | (
                InputDeclaration::Payload { .. },
                EvidenceSelection::WholePayload {} | EvidenceSelection::PayloadField { .. }
            ) | (
                InputDeclaration::ChangeSet {},
                EvidenceSelection::ChangeSetLiteralText { .. }
                    | EvidenceSelection::ChangeSetRegexText { .. }
                    | EvidenceSelection::ChangeSetAstPattern { .. }
                    | EvidenceSelection::ChangedUnits { .. }
                    | EvidenceSelection::RelatedUnits { .. }
            )
        );
        if !compatible {
            return Err(refusal(RuleDiagnosticCode::InputKindMismatch));
        }
        if let Some(selection) = declaration.select.conversation() {
            selection
                .validate()
                .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
        }
        match &declaration.select {
            EvidenceSelection::PayloadField { pointer } if !valid_pointer(pointer) => {
                return Err(refusal(RuleDiagnosticCode::InvalidConfiguration));
            }
            EvidenceSelection::LiteralText { text } if text.is_empty() => {
                return Err(refusal(RuleDiagnosticCode::InvalidConfiguration));
            }
            EvidenceSelection::RegexText { pattern } => {
                validate_pattern(&PresencePattern::Regex {
                    pattern: pattern.clone(),
                })?;
            }
            EvidenceSelection::AstPattern { language, pattern } => {
                validate_pattern(&PresencePattern::Ast {
                    language: language.clone(),
                    pattern: pattern.clone(),
                })?;
            }
            _ => {}
        }
    }
    Ok(())
}
pub(crate) fn validate_entries(
    entries: &[ApplicabilityEntry],
    inputs: &BTreeMap<SourceName, InputDeclaration>,
) -> Result<(), DiagnosticList> {
    for entry in entries {
        match entry {
            ApplicabilityEntry::Decision {
                question,
                evidence,
                threshold,
            } => {
                if !threshold_valid(*threshold) {
                    return Err(refusal(RuleDiagnosticCode::InvalidThreshold));
                }
                if !matches!(question, JudgmentQuestion::Noul { .. }) {
                    return Err(refusal(RuleDiagnosticCode::InputKindMismatch));
                }
                validate_question(question)?;
                validate_evidence(evidence, inputs)?;
            }
            ApplicabilityEntry::Condition { test } => match test {
                ConditionTest::OriginFieldEquals { value, .. } if value.is_empty() => {
                    return Err(refusal(RuleDiagnosticCode::InvalidConfiguration));
                }
                ConditionTest::OriginFieldEquals { .. } => {}
                ConditionTest::PayloadFieldEquals { input, pointer, .. }
                | ConditionTest::PayloadFieldIn { input, pointer, .. }
                | ConditionTest::PayloadFieldPresent { input, pointer } => {
                    if !valid_pointer(pointer) {
                        return Err(refusal(RuleDiagnosticCode::InvalidConfiguration));
                    }
                    if !matches!(inputs.get(input), Some(InputDeclaration::Payload { .. })) {
                        return Err(refusal(if inputs.contains_key(input) {
                            RuleDiagnosticCode::InputKindMismatch
                        } else {
                            RuleDiagnosticCode::UndeclaredInput
                        }));
                    }
                    if matches!(test,ConditionTest::PayloadFieldIn{values,..} if values.is_empty())
                    {
                        return Err(refusal(RuleDiagnosticCode::InvalidConfiguration));
                    }
                }
                ConditionTest::PatternPresent { input, pattern, .. } => {
                    if !matches!(
                        inputs.get(input),
                        Some(InputDeclaration::Document {} | InputDeclaration::Conversation {})
                    ) {
                        return Err(refusal(if inputs.contains_key(input) {
                            RuleDiagnosticCode::InputKindMismatch
                        } else {
                            RuleDiagnosticCode::UndeclaredInput
                        }));
                    }
                    validate_pattern(pattern)?;
                }
            },
        }
    }
    Ok(())
}
pub(crate) fn valid_pointer(pointer: &str) -> bool {
    if pointer.is_empty() {
        return true;
    }
    if !pointer.starts_with('/') {
        return false;
    }
    let mut chars = pointer.chars();
    while let Some(character) = chars.next() {
        if character == '~' && !matches!(chars.next(), Some('0' | '1')) {
            return false;
        }
    }
    true
}
fn validate_pattern(pattern: &PresencePattern) -> Result<(), DiagnosticList> {
    let invalid = || refusal(RuleDiagnosticCode::InvalidConfiguration);
    match pattern {
        PresencePattern::Literal { text } => {
            if text.is_empty() {
                return Err(invalid());
            }
        }
        PresencePattern::Regex { pattern } => {
            if pattern.is_empty() {
                return Err(invalid());
            }
            regex::Regex::new(pattern).map_err(|_| invalid())?;
        }
        PresencePattern::Ast { language, pattern } => {
            let language: ast_grep_language::SupportLang =
                language.parse().map_err(|_| invalid())?;
            ast_grep_core::Pattern::try_new(pattern, language).map_err(|_| invalid())?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "definition_admission_test.rs"]
mod tests;
