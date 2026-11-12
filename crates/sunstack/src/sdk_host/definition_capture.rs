//! Capture one authored or loaded configuration into an immutable definition bundle.
//!
//! Typed values are the execution authority. A captured value's original source is
//! parsed only to decide provenance: an unchanged value keeps its original origin and
//! bytes; a changed value executes as edited, with fresh normalized content.
use super::{
    matcher_transport::{
        AstMatcher, AstUtility, TransportRefusal, TransportRefusalKind, matcher_value,
        utility_value,
    },
    normalized_values::{canonical_value, normalized_text, source_id},
    sdk_contracts::{
        CapturedSource, CapturedValue, Configuration, DefinitionSourceFormat, Diagnostic,
        DiagnosticCode, EvidenceDefinitions, RuleEntries, RulebookDefinition, WholeSourceFormat,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
    path::{Path, PathBuf},
};
use sunstack_evidence::{
    CapturedFile, DefinitionOrigin, DefinitionSnapshot, EvidenceProjectDefinitions,
    MatcherDefinition, ProjectReadFailure, SnapshotAcquisition, SnapshotRepresentation,
    SourceAcquisitionFailure, SourceLocator, SourceUnavailable, UtilityDefinition,
};
use sunstack_lint::{
    ConfigLocation,
    configuration_loading::definition_bundle::{
        ConfigurationSettings, DefinitionBundle, EvidenceAcquisition, RootDefinition,
    },
};
use sunstack_rules::RulebookSource;

/// Why a configuration could not be captured, before any admission.
#[derive(Debug, Clone)]
pub enum CaptureFailure {
    /// An acquired or supplied definition was refused.
    Invalid(Vec<Diagnostic>),
    /// A referenced definition could not be read.
    Unavailable(Vec<Diagnostic>),
}

pub(crate) fn refusal(
    code: DiagnosticCode,
    origin: Option<&DefinitionOrigin>,
    json_path: Vec<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        code,
        origin: origin.cloned(),
        path: None,
        line: None,
        column: None,
        json_path,
        message: message.into(),
    }
}

pub(crate) fn unavailable(failure: &SourceAcquisitionFailure) -> Diagnostic {
    let code = match failure.reason {
        SourceUnavailable::NotFound => DiagnosticCode::SourceNotFound,
        SourceUnavailable::Unreadable => DiagnosticCode::SourceUnreadable,
        SourceUnavailable::Io => DiagnosticCode::SourceIo,
        SourceUnavailable::Encoding => DiagnosticCode::SourceEncoding,
    };
    refusal(
        code,
        Some(&DefinitionOrigin::file(&failure.path)),
        vec![],
        format!("{}: {}", failure.path.display(), failure.message),
    )
}

fn path_of(prefix: &[&str], rest: &[String]) -> Vec<String> {
    prefix
        .iter()
        .map(|segment| (*segment).to_owned())
        .chain(rest.iter().cloned())
        .collect()
}

fn transport_refusal(
    origin: Option<&DefinitionOrigin>,
    prefix: &[&str],
    refusal_detail: TransportRefusal,
) -> Diagnostic {
    let code = match refusal_detail.kind {
        TransportRefusalKind::Unrepresentable => DiagnosticCode::UnsupportedInputs,
        TransportRefusalKind::DeclaredId => DiagnosticCode::Syntax,
        TransportRefusalKind::Decoder if refusal_detail.message.starts_with("unknown field") => {
            DiagnosticCode::UnknownField
        }
        TransportRefusalKind::Decoder => DiagnosticCode::Syntax,
    };
    refusal(
        code,
        origin,
        path_of(prefix, &refusal_detail.json_path),
        refusal_detail.message,
    )
}

/// The settings every configuration supplies, from an SDK configuration.
pub(crate) fn settings_of(configuration: &Configuration) -> ConfigurationSettings {
    ConfigurationSettings {
        marker_prefix: configuration.marker_prefix.clone(),
        max_question_words: configuration
            .max_question_words
            .unwrap_or(NonZeroU32::new(400).unwrap_or(NonZeroU32::MIN)),
        check: configuration.check.clone(),
        decision_model: configuration.decision_model.clone(),
        limits: configuration.limits.clone(),
        filters: configuration.filters.clone(),
        kinds: configuration.evidence.kinds().clone(),
    }
}

/// Normalized root settings: what decides whether the root definition changed.
pub(crate) fn settings_value(settings: &ConfigurationSettings) -> serde_json::Value {
    canonical_value(serde_json::json!({
        "marker_prefix": settings.marker_prefix,
        "max_question_words": settings.max_question_words,
        "check": settings.check,
        "decision_model": settings.decision_model,
        "limits": settings.limits,
        "filters": settings.filters,
        "kinds": settings.kinds,
    }))
}

fn snapshot(
    origin: DefinitionOrigin,
    content: String,
    representation: SnapshotRepresentation,
    acquisition: SnapshotAcquisition,
) -> DefinitionSnapshot {
    DefinitionSnapshot::new(origin, content, representation, acquisition)
}

/// Snapshots, each distinct origin and content recorded once.
#[derive(Default)]
struct SnapshotReceipt {
    snapshots: Vec<DefinitionSnapshot>,
    seen: BTreeSet<(DefinitionOrigin, String)>,
}

impl SnapshotReceipt {
    fn record(&mut self, snapshot: DefinitionSnapshot) {
        if self
            .seen
            .insert((snapshot.origin.clone(), snapshot.sha256.clone()))
        {
            self.snapshots.push(snapshot);
        }
    }
}

/// Inline origin IDs must be unique within one bundle.
#[derive(Default)]
struct InlineIds(BTreeSet<String>);

impl InlineIds {
    fn claim(&mut self, id: &str, json_path: Vec<String>) -> Result<(), CaptureFailure> {
        if self.0.insert(id.to_owned()) {
            Ok(())
        } else {
            Err(CaptureFailure::Invalid(vec![refusal(
                DiagnosticCode::DuplicateEntry,
                None,
                json_path,
                format!("source ID {id} is used twice in this configuration"),
            )]))
        }
    }
}

fn inline_origin(id: String) -> DefinitionOrigin {
    DefinitionOrigin::Inline {
        label: id.clone(),
        id,
    }
}

fn edited(id: String, based_on: &SourceLocator) -> DefinitionOrigin {
    DefinitionOrigin::Edited {
        id,
        based_on: based_on.clone(),
    }
}

fn metadata_invalid(json_path: Vec<String>, message: impl Into<String>) -> Diagnostic {
    refusal(
        DiagnosticCode::CaptureMetadataInvalid,
        None,
        json_path,
        message,
    )
}

/// Definition files read during one call. Each path is read once and its bytes are
/// reused, so two references to one file always execute and report one revision.
#[derive(Default)]
pub(crate) struct CallFiles(BTreeMap<PathBuf, CapturedFile>);

impl CallFiles {
    pub(crate) fn read(&mut self, path: &Path) -> Result<CapturedFile, CaptureFailure> {
        if let Some(file) = self.0.get(path) {
            return Ok(file.clone());
        }
        let file = sunstack_evidence::read_definition_file(path)
            .map_err(|failure| CaptureFailure::Unavailable(vec![unavailable(&failure)]))?;
        self.0.insert(path.to_path_buf(), file.clone());
        Ok(file)
    }
}

/// Capture one configuration. Only definitions it references as files are read.
pub fn capture_configuration(
    configuration: &Configuration,
) -> Result<DefinitionBundle, CaptureFailure> {
    let invalid = |diagnostic: Diagnostic| CaptureFailure::Invalid(vec![diagnostic]);
    if configuration.id.is_empty() {
        return Err(invalid(refusal(
            DiagnosticCode::Syntax,
            None,
            vec!["id".into()],
            "configuration id must be nonempty",
        )));
    }
    if !configuration.base_directory.is_absolute() {
        return Err(invalid(refusal(
            DiagnosticCode::UnsupportedInputs,
            None,
            vec!["base_directory".into()],
            "base_directory must be an absolute path",
        )));
    }
    let base = &configuration.base_directory;
    let config_id = configuration.id.as_str();
    let mut receipt = SnapshotReceipt::default();
    let mut ids = InlineIds::default();
    let mut files = CallFiles::default();
    let settings = settings_of(configuration);
    let root = capture_root(configuration, &settings, &mut receipt, &mut ids)?;

    let mut rulebooks = Vec::new();
    for (index, rulebook) in configuration.rulebooks.iter().enumerate() {
        let index_text = index.to_string();
        let prefix = ["rulebooks", index_text.as_str()];
        rulebooks.push(Ok(capture_rulebook(
            config_id,
            base,
            rulebook,
            &prefix,
            &mut receipt,
            &mut ids,
            &mut files,
        )?));
    }

    let evidence = capture_evidence(
        config_id,
        base,
        &configuration.evidence,
        &root.origin,
        &mut receipt,
        &mut ids,
    )?;
    Ok(DefinitionBundle {
        inline_kind_origin: root.origin.clone(),
        root,
        settings,
        evidence,
        rulebooks,
        sources: receipt.snapshots,
    })
}

fn capture_root(
    configuration: &Configuration,
    settings: &ConfigurationSettings,
    receipt: &mut SnapshotReceipt,
    ids: &mut InlineIds,
) -> Result<RootDefinition, CaptureFailure> {
    let supplied = settings_value(settings);
    let label = configuration
        .label
        .clone()
        .unwrap_or_else(|| configuration.id.clone());
    let Some(original) = &configuration.original_config else {
        ids.claim(&configuration.id, vec!["id".into()])?;
        let origin = DefinitionOrigin::Inline {
            id: configuration.id.clone(),
            label,
        };
        receipt.record(snapshot(
            origin.clone(),
            normalized_text(&supplied),
            SnapshotRepresentation::NormalizedValue,
            SnapshotAcquisition::Inline,
        ));
        return Ok(RootDefinition {
            origin,
            text: None,
            base_directory: configuration.base_directory.clone(),
        });
    };
    let location = match &original.origin {
        SourceLocator::File { path } => ConfigLocation {
            file: path.clone(),
            directory: path
                .parent()
                .map_or_else(|| configuration.base_directory.clone(), Path::to_path_buf),
        },
        SourceLocator::Inline { .. } => ConfigLocation {
            file: PathBuf::from("sunstack.lint.json"),
            directory: configuration.base_directory.clone(),
        },
    };
    let parsed = sunstack_lint::parse_config(&original.content, &location).map_err(|error| {
        CaptureFailure::Invalid(vec![metadata_invalid(
            vec!["original_config".into(), "content".into()],
            format!("original configuration is malformed: {}", error.message),
        )])
    })?;
    let original_settings = settings_value(&ConfigurationSettings::from(&parsed));
    if original_settings == supplied {
        let origin = DefinitionOrigin::from(original.origin.clone());
        receipt.record(snapshot(
            origin.clone(),
            original.content.clone(),
            SnapshotRepresentation::OriginalText,
            SnapshotAcquisition::SuppliedSnapshot,
        ));
        return Ok(RootDefinition {
            origin,
            text: Some(original.content.clone()),
            base_directory: configuration.base_directory.clone(),
        });
    }
    let origin = edited(configuration.id.clone(), &original.origin);
    receipt.record(snapshot(
        origin.clone(),
        normalized_text(&supplied),
        SnapshotRepresentation::NormalizedValue,
        SnapshotAcquisition::Inline,
    ));
    Ok(RootDefinition {
        origin,
        text: None,
        base_directory: configuration.base_directory.clone(),
    })
}

/// Normalized typed rule entries: what decides whether captured entries changed.
fn normalized_entries(entries: &RuleEntries) -> Option<serde_json::Value> {
    serde_json::to_value(entries).ok().map(canonical_value)
}

fn entries_bytes(entries: &RuleEntries) -> Result<Vec<u8>, CaptureFailure> {
    serde_json::to_vec(entries).map_err(|error| {
        CaptureFailure::Invalid(vec![refusal(
            DiagnosticCode::UnsupportedInputs,
            None,
            vec![],
            format!("rule entries cannot be encoded: {error}"),
        )])
    })
}

fn capture_rulebook(
    config_id: &str,
    base: &Path,
    rulebook: &RulebookDefinition,
    prefix: &[&str],
    receipt: &mut SnapshotReceipt,
    ids: &mut InlineIds,
    files: &mut CallFiles,
) -> Result<RulebookSource, CaptureFailure> {
    let invalid = |diagnostic: Diagnostic| CaptureFailure::Invalid(vec![diagnostic]);
    match rulebook {
        RulebookDefinition::Files { markdown, entries } => {
            let markdown = files.read(&base.join(markdown))?;
            let entries = files.read(&base.join(entries))?;
            for file in [&markdown, &entries] {
                receipt.record(snapshot(
                    DefinitionOrigin::file(&file.path),
                    file.text.clone(),
                    SnapshotRepresentation::OriginalText,
                    SnapshotAcquisition::Read,
                ));
            }
            Ok(RulebookSource {
                markdown_origin: DefinitionOrigin::file(&markdown.path),
                markdown: markdown.text,
                entries_origin: DefinitionOrigin::file(&entries.path),
                entries: entries.text.into_bytes(),
            })
        }
        RulebookDefinition::Inline {
            id,
            markdown,
            entries,
        } => {
            if id.is_empty() {
                return Err(invalid(refusal(
                    DiagnosticCode::Syntax,
                    None,
                    path_of(prefix, &["id".into()]),
                    "inline rulebook id must be nonempty",
                )));
            }
            ids.claim(id, path_of(prefix, &["id".into()]))?;
            let entries_id = source_id(config_id, &[prefix[0], prefix[1], "entries"]);
            ids.claim(&entries_id, path_of(prefix, &["entries".into()]))?;
            let markdown_origin = inline_origin(id.clone());
            let entries_origin = inline_origin(entries_id);
            receipt.record(snapshot(
                markdown_origin.clone(),
                markdown.clone(),
                SnapshotRepresentation::OriginalText,
                SnapshotAcquisition::Inline,
            ));
            let bytes = entries_bytes(entries)?;
            receipt.record(snapshot(
                entries_origin.clone(),
                normalized_text(&normalized_entries(entries).unwrap_or_default()),
                SnapshotRepresentation::NormalizedValue,
                SnapshotAcquisition::Inline,
            ));
            Ok(RulebookSource {
                markdown_origin,
                markdown: markdown.clone(),
                entries_origin,
                entries: bytes,
            })
        }
        RulebookDefinition::Captured {
            id: _,
            markdown,
            entries,
        } => {
            let (markdown_origin, markdown_text) =
                capture_markdown(config_id, prefix, markdown, receipt)?;
            let (entries_origin, entries) = capture_entries(config_id, prefix, entries, receipt)?;
            Ok(RulebookSource {
                markdown_origin,
                markdown: markdown_text,
                entries_origin,
                entries,
            })
        }
    }
}

fn whole_original(
    original: &CapturedSource,
    format: WholeSourceFormat,
    json_path: Vec<String>,
) -> Result<(&SourceLocator, &str), CaptureFailure> {
    match original {
        CapturedSource::Whole {
            format: actual,
            origin,
            content,
        } if *actual == format => Ok((origin, content)),
        _ => Err(CaptureFailure::Invalid(vec![metadata_invalid(
            json_path,
            format!("captured source must have format {format:?}"),
        )])),
    }
}

fn capture_markdown(
    config_id: &str,
    prefix: &[&str],
    markdown: &CapturedValue<String>,
    receipt: &mut SnapshotReceipt,
) -> Result<(DefinitionOrigin, String), CaptureFailure> {
    let (origin, content) = whole_original(
        &markdown.original,
        WholeSourceFormat::Markdown,
        path_of(prefix, &["markdown".into(), "original".into()]),
    )?;
    let (origin, acquisition) = if markdown.value == content {
        (
            DefinitionOrigin::from(origin.clone()),
            SnapshotAcquisition::SuppliedSnapshot,
        )
    } else {
        (
            edited(
                source_id(config_id, &[prefix[0], prefix[1], "markdown"]),
                origin,
            ),
            SnapshotAcquisition::Inline,
        )
    };
    receipt.record(snapshot(
        origin.clone(),
        markdown.value.clone(),
        SnapshotRepresentation::OriginalText,
        acquisition,
    ));
    Ok((origin, markdown.value.clone()))
}

fn capture_entries(
    config_id: &str,
    prefix: &[&str],
    entries: &CapturedValue<RuleEntries>,
    receipt: &mut SnapshotReceipt,
) -> Result<(DefinitionOrigin, Vec<u8>), CaptureFailure> {
    let original_path = path_of(prefix, &["entries".into(), "original".into()]);
    let (origin, content) = whole_original(
        &entries.original,
        WholeSourceFormat::RuleEntriesJson,
        original_path.clone(),
    )?;
    let original_origin = DefinitionOrigin::from(origin.clone());
    let decoded = sunstack_rules::decode_rule_entries(content.as_bytes(), &original_origin)
        .ok()
        .and_then(|decoded| {
            let mut entries = RuleEntries::new();
            for (key, entry) in decoded {
                if entries.insert(key, entry.ok()?).is_some() {
                    return None;
                }
            }
            normalized_entries(&entries)
        })
        .ok_or_else(|| {
            CaptureFailure::Invalid(vec![metadata_invalid(
                original_path,
                "original rule entries are malformed",
            )])
        })?;
    if normalized_entries(&entries.value).as_ref() == Some(&decoded) {
        receipt.record(snapshot(
            original_origin.clone(),
            content.to_owned(),
            SnapshotRepresentation::OriginalText,
            SnapshotAcquisition::SuppliedSnapshot,
        ));
        return Ok((original_origin, content.as_bytes().to_vec()));
    }
    let origin = edited(
        source_id(config_id, &[prefix[0], prefix[1], "entries"]),
        origin,
    );
    let bytes = entries_bytes(&entries.value)?;
    receipt.record(snapshot(
        origin.clone(),
        normalized_text(&normalized_entries(&entries.value).unwrap_or_default()),
        SnapshotRepresentation::NormalizedValue,
        SnapshotAcquisition::Inline,
    ));
    Ok((origin, bytes))
}

fn capture_evidence(
    config_id: &str,
    base: &Path,
    evidence: &EvidenceDefinitions,
    root_origin: &DefinitionOrigin,
    receipt: &mut SnapshotReceipt,
    ids: &mut InlineIds,
) -> Result<EvidenceAcquisition, CaptureFailure> {
    let invalid = |diagnostic: Diagnostic| CaptureFailure::Invalid(vec![diagnostic]);
    let mut definitions = EvidenceProjectDefinitions {
        project: root_origin.clone(),
        rule_directories: vec![],
        utilities: vec![],
        matchers: vec![],
        inputs: vec![],
    };
    match evidence {
        EvidenceDefinitions::Project { config_path, .. } => {
            let files = sunstack_evidence::read_evidence_project(&base.join(config_path)).map_err(
                |failure| match failure {
                    ProjectReadFailure::Unavailable(failure) => {
                        CaptureFailure::Unavailable(vec![unavailable(&failure.source)])
                    }
                    ProjectReadFailure::Refused(refusal_detail) => {
                        CaptureFailure::Invalid(vec![project_refusal(&refusal_detail)])
                    }
                },
            )?;
            let decoded = sunstack_evidence::decode_evidence_project(&files).map_err(
                |failure| match failure {
                    ProjectReadFailure::Unavailable(failure) => {
                        CaptureFailure::Unavailable(vec![unavailable(&failure.source)])
                    }
                    ProjectReadFailure::Refused(refusal_detail) => {
                        CaptureFailure::Invalid(vec![project_refusal(&refusal_detail)])
                    }
                },
            )?;
            for file in std::iter::once(&files.project).chain(
                files
                    .utility_files
                    .iter()
                    .chain(&files.rule_files)
                    .filter_map(|file| file.as_ref().ok()),
            ) {
                receipt.record(snapshot(
                    DefinitionOrigin::file(&file.path),
                    file.text.clone(),
                    SnapshotRepresentation::OriginalText,
                    SnapshotAcquisition::Read,
                ));
            }
            return Ok(EvidenceAcquisition::Definitions(decoded));
        }
        EvidenceDefinitions::Inline {
            matchers,
            utilities,
            ..
        } => {
            for (name, value) in utilities {
                let prefix = ["evidence", "utilities", name.as_str()];
                let id = source_id(config_id, &prefix);
                ids.claim(&id, path_of(&prefix, &[]))?;
                let origin = inline_origin(id);
                let utility = value.upstream(name);
                let canonical = utility_value(&utility)
                    .map_err(|detail| invalid(transport_refusal(Some(&origin), &prefix, detail)))?;
                receipt.record(snapshot(
                    origin.clone(),
                    normalized_text(&canonical),
                    SnapshotRepresentation::NormalizedValue,
                    SnapshotAcquisition::Inline,
                ));
                definitions
                    .utilities
                    .push(UtilityDefinition { origin, utility });
            }
            for (name, value) in matchers {
                let prefix = ["evidence", "matchers", name.as_str()];
                let id = source_id(config_id, &prefix);
                ids.claim(&id, path_of(&prefix, &[]))?;
                let origin = inline_origin(id);
                let matcher = value.upstream(name);
                let canonical = matcher_value(&matcher)
                    .map_err(|detail| invalid(transport_refusal(Some(&origin), &prefix, detail)))?;
                receipt.record(snapshot(
                    origin.clone(),
                    normalized_text(&canonical),
                    SnapshotRepresentation::NormalizedValue,
                    SnapshotAcquisition::Inline,
                ));
                definitions
                    .matchers
                    .push(MatcherDefinition { origin, matcher });
            }
        }
        EvidenceDefinitions::Captured {
            matchers,
            utilities,
            ..
        } => {
            for (name, captured) in utilities {
                let prefix = ["evidence", "utilities", name.as_str()];
                let (origin, utility) = capture_definition(
                    config_id,
                    &prefix,
                    name,
                    captured,
                    DefinitionSourceFormat::UtilityYaml,
                    receipt,
                    sunstack_evidence::decode_utility_file,
                    |utility| utility.id.as_str(),
                    |name, value: &AstUtility| value.upstream(name),
                    utility_value,
                )?;
                definitions
                    .utilities
                    .push(UtilityDefinition { origin, utility });
            }
            for (name, captured) in matchers {
                let prefix = ["evidence", "matchers", name.as_str()];
                let (origin, matcher) = capture_definition(
                    config_id,
                    &prefix,
                    name,
                    captured,
                    DefinitionSourceFormat::MatcherYaml,
                    receipt,
                    sunstack_evidence::decode_matcher_file,
                    |matcher| matcher.id.as_str(),
                    |name, value: &AstMatcher| value.upstream(name),
                    matcher_value,
                )?;
                definitions
                    .matchers
                    .push(MatcherDefinition { origin, matcher });
            }
        }
    }
    Ok(EvidenceAcquisition::Definitions(definitions))
}

pub(crate) fn project_refusal(detail: &sunstack_evidence::ProjectRefusal) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::Syntax,
        origin: Some(detail.definition.as_ref().clone()),
        path: None,
        line: u32::try_from(detail.line).ok(),
        column: detail.column.and_then(|column| u32::try_from(column).ok()),
        json_path: vec![],
        message: detail.message.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
fn capture_definition<TDefinition, TTransport>(
    config_id: &str,
    prefix: &[&str],
    name: &str,
    captured: &CapturedValue<TTransport>,
    format: DefinitionSourceFormat,
    receipt: &mut SnapshotReceipt,
    decode_file: impl Fn(&CapturedFile) -> Result<Vec<TDefinition>, sunstack_evidence::ProjectRefusal>,
    definition_id: impl Fn(&TDefinition) -> &str,
    named_upstream: impl Fn(&str, &TTransport) -> TDefinition,
    canonical: impl Fn(&TDefinition) -> Result<serde_json::Value, TransportRefusal>,
) -> Result<(DefinitionOrigin, TDefinition), CaptureFailure> {
    let original_path = path_of(prefix, &["original".into()]);
    let metadata = |message: &str| {
        CaptureFailure::Invalid(vec![metadata_invalid(original_path.clone(), message)])
    };
    let CapturedSource::Definition {
        format: actual,
        origin,
        content,
        definition_id: wanted,
    } = &captured.original
    else {
        return Err(metadata("captured definition needs a definition_id"));
    };
    if *actual != format {
        return Err(metadata(&format!(
            "captured source must have format {format:?}"
        )));
    }
    let file = CapturedFile {
        path: match origin {
            SourceLocator::File { path } => path.clone(),
            SourceLocator::Inline { id, .. } => PathBuf::from(id),
        },
        text: content.clone(),
    };
    let mut candidates = decode_file(&file)
        .map_err(|_| metadata("original definition source is malformed"))?
        .into_iter()
        .filter(|definition| definition_id(definition) == wanted)
        .collect::<Vec<_>>();
    if candidates.len() != 1 {
        return Err(metadata(
            "definition_id must name exactly one original definition",
        ));
    }
    let original = candidates.remove(0);
    let original_value =
        canonical(&original).map_err(|_| metadata("original definition is unrepresentable"))?;
    let edited_id = source_id(config_id, prefix);
    let definition = named_upstream(name, &captured.value);
    let supplied_value = canonical(&definition).ok();
    if name == wanted && supplied_value.as_ref() == Some(&original_value) {
        let origin = DefinitionOrigin::from(origin.clone());
        receipt.record(snapshot(
            origin.clone(),
            content.clone(),
            SnapshotRepresentation::OriginalText,
            SnapshotAcquisition::SuppliedSnapshot,
        ));
        return Ok((origin, original));
    }
    let edited_origin = edited(edited_id, origin);
    let value = canonical(&definition).map_err(|detail| {
        CaptureFailure::Invalid(vec![transport_refusal(
            Some(&edited_origin),
            prefix,
            detail,
        )])
    })?;
    receipt.record(snapshot(
        edited_origin.clone(),
        normalized_text(&value),
        SnapshotRepresentation::NormalizedValue,
        SnapshotAcquisition::Inline,
    ));
    Ok((edited_origin, definition))
}
