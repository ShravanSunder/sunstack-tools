use crate::{
    DiagnosticList, RuleDiagnostic, RuleDiagnosticCode, RuleEntryWire, RuleId, RulebookPair,
    RulebookRule, RulebookSentence, admit_rule_entry,
    rule_definitions::strict_decoding::decode_definition,
};
use schemars::JsonSchema;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{MapAccess, Visitor},
};
use serde_json::value::RawValue;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::{Path, PathBuf},
};
use sunstack_evidence::{
    EvidenceKindDefinition, EvidenceKindName, EvidenceProject, MarkdownDocument,
    MarkdownMarkerBinding, MarkerPrefix,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RulebookSkippedRule {
    pub markdown: PathBuf,
    pub rules: PathBuf,
    pub key: String,
    pub rule_id: Option<RuleId>,
    pub diagnostics: DiagnosticList,
}
#[derive(Debug, Clone)]
pub struct RulebookAssembly {
    pub rules: Vec<RulebookRule>,
    pub skipped: Vec<RulebookSkippedRule>,
}

struct RawEntries<'a>(Vec<(String, &'a RawValue)>);
impl<'de> Deserialize<'de> for RawEntries<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EntriesVisitor;
        impl<'de> Visitor<'de> for EntriesVisitor {
            type Value = RawEntries<'de>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("one object of rule entries")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry()? {
                    entries.push(entry);
                }
                Ok(RawEntries(entries))
            }
        }
        deserializer.deserialize_map(EntriesVisitor)
    }
}

fn diagnostic(
    file: &Path,
    line: usize,
    column: usize,
    code: RuleDiagnosticCode,
    message: &str,
) -> DiagnosticList {
    let mut diagnostic = RuleDiagnostic::new(code).definition(file, line, column, vec![]);
    diagnostic.guidance = message.into();
    diagnostic.into()
}

pub fn assemble_rulebook(
    pairs: &[RulebookPair],
    prefix: &MarkerPrefix,
    max_question_words: usize,
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    project: &EvidenceProject,
) -> Result<RulebookAssembly, DiagnosticList> {
    let mut assembly = RulebookAssembly {
        rules: vec![],
        skipped: vec![],
    };
    for pair in pairs {
        let markdown = std::fs::read_to_string(&pair.markdown).map_err(|_| {
            diagnostic(
                &pair.markdown,
                1,
                1,
                RuleDiagnosticCode::InvalidConfiguration,
                "rulebook is unreadable",
            )
        })?;
        let bytes = std::fs::read(&pair.rules).map_err(|_| {
            diagnostic(
                &pair.rules,
                1,
                1,
                RuleDiagnosticCode::InvalidConfiguration,
                "rule file is unreadable",
            )
        })?;
        let mut decoder = serde_json::Deserializer::from_slice(&bytes);
        let raw = RawEntries::deserialize(&mut decoder).map_err(|error| {
            diagnostic(
                &pair.rules,
                error.line(),
                error.column(),
                RuleDiagnosticCode::MalformedJson,
                "rule file must be a JSON object",
            )
        })?;
        decoder.end().map_err(|error| {
            diagnostic(
                &pair.rules,
                error.line(),
                error.column(),
                RuleDiagnosticCode::MalformedJson,
                "unexpected trailing rule-file content",
            )
        })?;
        let mut entries = BTreeMap::<String, Vec<&RawValue>>::new();
        for (key, value) in raw.0 {
            entries.entry(key).or_default().push(value);
        }
        let bindings = MarkdownDocument::parse(&markdown).bind_markers(prefix, max_question_words);
        let mut markers = BTreeMap::<String, Vec<MarkdownMarkerBinding>>::new();
        let mut order = Vec::new();
        for binding in bindings {
            let marker = match &binding {
                MarkdownMarkerBinding::Bound { binding } => &binding.marker,
                MarkdownMarkerBinding::Refused { marker, .. } => marker,
            };
            let key = marker.id.as_str().to_owned();
            if !markers.contains_key(&key) {
                order.push(key.clone());
            }
            markers.entry(key).or_default().push(binding);
        }
        let mut seen = BTreeSet::new();
        for key in order.into_iter().chain(entries.keys().cloned()) {
            if !seen.insert(key.clone()) {
                continue;
            }
            let mut errors = DiagnosticList::default();
            let bound = match markers.get(&key).and_then(|values| values.first()) {
                Some(MarkdownMarkerBinding::Bound { binding }) => Some(binding),
                Some(MarkdownMarkerBinding::Refused { marker, reason }) => {
                    errors.extend(diagnostic(
                        &pair.markdown,
                        marker.span.start_line,
                        marker.span.start_column,
                        RuleDiagnosticCode::InvalidConfiguration,
                        reason.guidance(),
                    ));
                    None
                }
                None => {
                    errors.extend(diagnostic(
                        &pair.rules,
                        1,
                        1,
                        RuleDiagnosticCode::OrphanedRuleEntry,
                        "entry has no rulebook marker",
                    ));
                    None
                }
            };
            let mut entry = None;
            let mut entry_line = 1;
            if let Some(values) = entries.get(&key) {
                if values.len() != 1 {
                    errors.extend(diagnostic(
                        &pair.rules,
                        1,
                        1,
                        RuleDiagnosticCode::DuplicateField,
                        "marker has duplicate rule entries",
                    ));
                }
                if let Some(raw) = values.first() {
                    let offset = raw
                        .get()
                        .as_ptr()
                        .addr()
                        .saturating_sub(bytes.as_ptr().addr())
                        .min(bytes.len());
                    entry_line = bytes[..offset].iter().filter(|b| **b == b'\n').count() + 1;
                    match decode_definition::<RuleEntryWire>(
                        raw.get().as_bytes(),
                        &pair.rules,
                        entry_line,
                    ) {
                        Ok(value) => entry = Some(value),
                        Err(refusals) => errors.extend(refusals),
                    }
                }
            } else {
                errors.extend(diagnostic(
                    &pair.markdown,
                    bound.map_or(1, |b| b.marker.span.start_line),
                    1,
                    RuleDiagnosticCode::MissingRuleEntry,
                    "rulebook marker has no entry",
                ));
            }
            let graph = if let Some(entry) = &entry {
                match admit_rule_entry(entry, kinds, project) {
                    Ok(graph) => Some(graph),
                    Err(refusals) => {
                        for refusal in refusals {
                            errors.extend(diagnostic(
                                &pair.rules,
                                entry_line,
                                1,
                                RuleDiagnosticCode::InvalidConfiguration,
                                &format!("{}: {}", refusal.field_path, refusal.message),
                            ));
                        }
                        None
                    }
                }
            } else {
                None
            };
            match (bound, entry, graph) {
                (Some(binding), Some(entry), Some(graph)) if errors.items.is_empty() => {
                    assembly.rules.push(RulebookRule {
                        name: entry.name,
                        sentence: RulebookSentence {
                            marker: binding.marker.id.clone(),
                            rulebook: pair.markdown.clone(),
                            line: binding.span.start_line,
                            standard: binding.question_text.clone(),
                        },
                        kind: entry.evidence,
                        questions: entry.questions,
                        decision: graph,
                        severity: entry.severity,
                    })
                }
                (_, entry, _) => assembly.skipped.push(RulebookSkippedRule {
                    markdown: pair.markdown.clone(),
                    rules: pair.rules.clone(),
                    key,
                    rule_id: entry.map(|e| e.name),
                    diagnostics: errors,
                }),
            }
        }
    }
    let mut counts = BTreeMap::<RuleId, usize>::new();
    for rule in &assembly.rules {
        *counts.entry(rule.name.clone()).or_default() += 1;
    }
    let mut retained = Vec::new();
    for rule in assembly.rules {
        if counts.get(&rule.name).copied().unwrap_or_default() > 1 {
            assembly.skipped.push(RulebookSkippedRule {
                markdown: rule.sentence.rulebook.clone(),
                rules: pairs
                    .iter()
                    .find(|pair| pair.markdown == rule.sentence.rulebook)
                    .map(|pair| pair.rules.clone())
                    .unwrap_or_default(),
                key: rule.sentence.marker.as_str().into(),
                rule_id: Some(rule.name),
                diagnostics: diagnostic(
                    &rule.sentence.rulebook,
                    rule.sentence.line,
                    1,
                    RuleDiagnosticCode::InvalidConfiguration,
                    "rule name must be unique across the config",
                ),
            });
        } else {
            retained.push(rule);
        }
    }
    assembly.rules = retained;
    Ok(assembly)
}

#[cfg(test)]
mod tests;
