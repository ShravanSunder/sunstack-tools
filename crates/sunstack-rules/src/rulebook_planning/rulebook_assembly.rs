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
    DefinitionOrigin, EvidenceKindDefinition, EvidenceKindName, EvidenceProject, MarkdownDocument,
    MarkdownMarkerBinding, MarkerPrefix, SourceAcquisitionFailure,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RulebookSkippedRule {
    #[serde(with = "sunstack_evidence::cli_path")]
    #[schemars(with = "PathBuf")]
    pub markdown: DefinitionOrigin,
    #[serde(with = "sunstack_evidence::cli_path")]
    #[schemars(with = "PathBuf")]
    pub rules: DefinitionOrigin,
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
    definition: &DefinitionOrigin,
    line: usize,
    column: usize,
    code: RuleDiagnosticCode,
    message: &str,
) -> DiagnosticList {
    let mut diagnostic =
        RuleDiagnostic::new(code).definition(&definition.cli_path(), line, column, vec![]);
    diagnostic.guidance = message.into();
    let mut list = DiagnosticList::from(diagnostic);
    list.attribute_definition(definition);
    list
}

/// One rulebook's Markdown and rule entries, captured once and independent of files.
#[derive(Debug, Clone)]
pub struct RulebookSource {
    pub markdown_origin: DefinitionOrigin,
    pub markdown: String,
    pub entries_origin: DefinitionOrigin,
    /// The rule-entry JSON object: exact file bytes, or the normalized typed value.
    pub entries: Vec<u8>,
}

/// A rulebook file pair that could not be read.
#[derive(Debug, Clone)]
pub struct RulebookReadFailure {
    pub source: SourceAcquisitionFailure,
    /// The existing CLI refusal for this read.
    pub diagnostics: DiagnosticList,
}

/// Read one configured Markdown/rule-file pair exactly once.
pub fn read_rulebook_pair(pair: &RulebookPair) -> Result<RulebookSource, RulebookReadFailure> {
    let unreadable = |path: &Path, message: &str, error: std::io::Error| {
        let origin = DefinitionOrigin::file(path);
        RulebookReadFailure {
            source: SourceAcquisitionFailure {
                path: path.to_path_buf(),
                reason: match error.kind() {
                    std::io::ErrorKind::NotFound => sunstack_evidence::SourceUnavailable::NotFound,
                    std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::IsADirectory => {
                        sunstack_evidence::SourceUnavailable::Unreadable
                    }
                    std::io::ErrorKind::InvalidData => {
                        sunstack_evidence::SourceUnavailable::Encoding
                    }
                    _ => sunstack_evidence::SourceUnavailable::Io,
                },
                message: error.to_string(),
            },
            diagnostics: diagnostic(
                &origin,
                1,
                1,
                RuleDiagnosticCode::InvalidConfiguration,
                message,
            ),
        }
    };
    let markdown = std::fs::read_to_string(&pair.markdown)
        .map_err(|error| unreadable(&pair.markdown, "rulebook is unreadable", error))?;
    let entries = std::fs::read(&pair.rules)
        .map_err(|error| unreadable(&pair.rules, "rule file is unreadable", error))?;
    Ok(RulebookSource {
        markdown_origin: DefinitionOrigin::file(&pair.markdown),
        markdown,
        entries_origin: DefinitionOrigin::file(&pair.rules),
        entries,
    })
}

/// One keyed rule entry and its own strict-decoding result.
pub type DecodedRuleEntry = (String, Result<RuleEntryWire, DiagnosticList>);

/// Decode a rule-entry JSON object into its entries with the strict entry decoder,
/// in authored order. Each entry keeps its own refusal; duplicate keys are kept.
pub fn decode_rule_entries(
    bytes: &[u8],
    origin: &DefinitionOrigin,
) -> Result<Vec<DecodedRuleEntry>, DiagnosticList> {
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let raw = RawEntries::deserialize(&mut decoder).map_err(|error| {
        diagnostic(
            origin,
            error.line(),
            error.column(),
            RuleDiagnosticCode::MalformedJson,
            "rule file must be a JSON object",
        )
    })?;
    decoder.end().map_err(|error| {
        diagnostic(
            origin,
            error.line(),
            error.column(),
            RuleDiagnosticCode::MalformedJson,
            "unexpected trailing rule-file content",
        )
    })?;
    Ok(raw
        .0
        .into_iter()
        .map(|(key, raw)| {
            let offset = raw
                .get()
                .as_ptr()
                .addr()
                .saturating_sub(bytes.as_ptr().addr())
                .min(bytes.len());
            let line = bytes[..offset].iter().filter(|b| **b == b'\n').count() + 1;
            let decoded =
                decode_definition::<RuleEntryWire>(raw.get().as_bytes(), &origin.cli_path(), line)
                    .map_err(|mut refusals| {
                        refusals.attribute_definition(origin);
                        refusals
                    });
            (key, decoded)
        })
        .collect())
}

pub fn assemble_rulebook(
    pairs: &[RulebookPair],
    prefix: &MarkerPrefix,
    max_question_words: usize,
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    project: &EvidenceProject,
    filters: &crate::FilterRegistry,
) -> Result<RulebookAssembly, DiagnosticList> {
    let sources = pairs
        .iter()
        .map(|pair| read_rulebook_pair(pair).map_err(|failure| failure.diagnostics))
        .collect::<Vec<_>>();
    assemble_rulebook_sources(
        &sources,
        prefix,
        max_question_words,
        kinds,
        project,
        filters,
    )
}

/// Bind markers and admit entries for captured rulebooks with the shared policy.
///
/// Sources are visited in order; the first unread source refuses the whole assembly,
/// as file reading did.
pub fn assemble_rulebook_sources(
    sources: &[Result<RulebookSource, DiagnosticList>],
    prefix: &MarkerPrefix,
    max_question_words: usize,
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    project: &EvidenceProject,
    filters: &crate::FilterRegistry,
) -> Result<RulebookAssembly, DiagnosticList> {
    let mut assembly = RulebookAssembly {
        rules: vec![],
        skipped: vec![],
    };
    for source in sources {
        let source = source.as_ref().map_err(Clone::clone)?;
        let markdown = &source.markdown;
        let bytes = &source.entries;
        let markdown_origin = &source.markdown_origin;
        let rules_origin = &source.entries_origin;
        let mut decoder = serde_json::Deserializer::from_slice(bytes);
        let raw = RawEntries::deserialize(&mut decoder).map_err(|error| {
            diagnostic(
                rules_origin,
                error.line(),
                error.column(),
                RuleDiagnosticCode::MalformedJson,
                "rule file must be a JSON object",
            )
        })?;
        decoder.end().map_err(|error| {
            diagnostic(
                rules_origin,
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
        let bindings = MarkdownDocument::parse(markdown).bind_markers(prefix, max_question_words);
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
                        markdown_origin,
                        marker.span.start_line,
                        marker.span.start_column,
                        RuleDiagnosticCode::InvalidConfiguration,
                        reason.guidance(),
                    ));
                    None
                }
                None => {
                    errors.extend(diagnostic(
                        rules_origin,
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
                        rules_origin,
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
                        &rules_origin.cli_path(),
                        entry_line,
                    ) {
                        Ok(value) => entry = Some(value),
                        Err(mut refusals) => {
                            refusals.attribute_definition(rules_origin);
                            errors.extend(refusals)
                        }
                    }
                }
            } else {
                errors.extend(diagnostic(
                    markdown_origin,
                    bound.map_or(1, |b| b.marker.span.start_line),
                    1,
                    RuleDiagnosticCode::MissingRuleEntry,
                    "rulebook marker has no entry",
                ));
            }
            let graph = if let Some(entry) = &entry {
                match admit_rule_entry(entry, kinds, project, filters) {
                    Ok(graph) => Some(graph),
                    Err(refusals) => {
                        for refusal in refusals {
                            errors.extend(diagnostic(
                                rules_origin,
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
                        reference: entry.reference,
                        gate: entry
                            .only_when
                            .as_ref()
                            .and_then(|wire| crate::admit_only_when(wire, filters).ok()),
                        name: entry.name,
                        sentence: RulebookSentence {
                            marker: binding.marker.id.clone(),
                            rulebook: markdown_origin.clone(),
                            line: binding.span.start_line,
                            standard: binding.question_text.clone(),
                        },
                        kind: entry.evidence,
                        questions: entry.questions,
                        decision: graph,
                        severity: entry.severity,
                        related: entry.related,
                    })
                }
                (_, entry, _) => assembly.skipped.push(RulebookSkippedRule {
                    markdown: markdown_origin.clone(),
                    rules: rules_origin.clone(),
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
                rules: sources
                    .iter()
                    .filter_map(|source| source.as_ref().ok())
                    .find(|source| source.markdown_origin == rule.sentence.rulebook)
                    .map(|source| source.entries_origin.clone())
                    .unwrap_or_else(|| rule.sentence.rulebook.clone()),
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
#[path = "rulebook_assembly_test.rs"]
mod tests;
