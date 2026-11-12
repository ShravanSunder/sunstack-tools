//! Rust-owned SDK contracts: authored and captured configuration, call outcomes and
//! diagnostics. TypeScript declarations are generated from these types.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, num::NonZeroU32, path::PathBuf};
use sunstack_evidence::{
    DefinitionOrigin, DefinitionSnapshot, EvidenceKindDefinition, EvidenceKindName, MarkerPrefix,
    SourceLocator,
};
use sunstack_lint::{CheckedFiles, DecisionModelSettings, LintInvalidCode, LintLimits};
use sunstack_rules::{FilterName, FilterWire, RuleEntryWire};

use super::matcher_transport::{AstMatcher, AstUtility};

/// The SDK contract revision; bumped whenever a generated declaration changes meaning.
pub const CONTRACT_REVISION: u32 = 5;

/// Which original decoder a captured source needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum WholeSourceFormat {
    Markdown,
    RuleEntriesJson,
}

/// Which original decoder a captured definition inside a multi-definition YAML needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum DefinitionSourceFormat {
    MatcherYaml,
    UtilityYaml,
}

/// The exact original text a captured value was decoded from: provenance only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum CapturedSource {
    Whole {
        format: WholeSourceFormat,
        origin: SourceLocator,
        content: String,
    },
    Definition {
        format: DefinitionSourceFormat,
        origin: SourceLocator,
        content: String,
        definition_id: String,
    },
}

/// An editable value plus the original source it was captured from.
///
/// `value` is the only execution authority; `original` is compared to decide whether
/// the value still matches its source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct CapturedValue<TDefinition> {
    pub value: TDefinition,
    pub original: CapturedSource,
}

/// Rule entries keyed by their rulebook marker.
pub type RuleEntries = BTreeMap<String, RuleEntryWire>;

/// One rulebook: referenced files, inline values, or values captured by `load`.
///
/// Entries and matchers are decoded by the same strict decoders the file path uses;
/// a dynamically invalid value is refused when the configuration is decoded.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
// Transport values are decoded once per call; boxing would only add indirection.
#[allow(clippy::large_enum_variant)]
pub enum RulebookDefinition {
    Files {
        markdown: PathBuf,
        entries: PathBuf,
    },
    Inline {
        id: String,
        markdown: String,
        entries: RuleEntries,
    },
    Captured {
        id: String,
        markdown: CapturedValue<String>,
        entries: CapturedValue<RuleEntries>,
    },
}

/// Evidence definitions: inline values, values captured by `load`, or an ast-grep project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[allow(clippy::large_enum_variant)]
pub enum EvidenceDefinitions {
    Inline {
        matchers: BTreeMap<String, AstMatcher>,
        #[serde(default)]
        utilities: BTreeMap<String, AstUtility>,
        kinds: BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    },
    Captured {
        matchers: BTreeMap<String, CapturedValue<AstMatcher>>,
        utilities: BTreeMap<String, CapturedValue<AstUtility>>,
        kinds: BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    },
    Project {
        config_path: PathBuf,
        kinds: BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    },
}

impl EvidenceDefinitions {
    pub fn kinds(&self) -> &BTreeMap<EvidenceKindName, EvidenceKindDefinition> {
        match self {
            Self::Inline { kinds, .. }
            | Self::Captured { kinds, .. }
            | Self::Project { kinds, .. } => kinds,
        }
    }
}

/// The root settings' original source text, when the configuration was loaded.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct OriginalConfiguration {
    pub origin: SourceLocator,
    pub content: String,
}

/// One complete configuration: the single execution authority for a call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct Configuration {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub label: Option<String>,
    /// Absolute base for definition references; never the checked root.
    pub base_directory: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub original_config: Option<OriginalConfiguration>,
    #[serde(default, rename = "$schema", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub schema: Option<String>,
    pub marker_prefix: MarkerPrefix,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub max_question_words: Option<NonZeroU32>,
    pub check: CheckedFiles,
    pub decision_model: DecisionModelSettings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub limits: Option<LintLimits>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub filters: BTreeMap<FilterName, FilterWire>,
    pub rulebooks: Vec<RulebookDefinition>,
    pub evidence: EvidenceDefinitions,
}

/// A rulebook captured by `load`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct CapturedRulebook {
    pub kind: CapturedKind,
    pub id: String,
    pub markdown: CapturedValue<String>,
    pub entries: CapturedValue<RuleEntries>,
}

/// Evidence captured by `load`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct CapturedEvidence {
    pub kind: CapturedKind,
    pub matchers: BTreeMap<String, CapturedValue<AstMatcher>>,
    pub utilities: BTreeMap<String, CapturedValue<AstUtility>>,
    pub kinds: BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
}

/// The literal `captured` discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum CapturedKind {
    Captured,
}

/// A configuration whose definitions were all captured by `load`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct ResolvedConfiguration {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub label: Option<String>,
    pub base_directory: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub original_config: Option<OriginalConfiguration>,
    #[serde(default, rename = "$schema", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub schema: Option<String>,
    pub marker_prefix: MarkerPrefix,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub max_question_words: Option<NonZeroU32>,
    pub check: CheckedFiles,
    pub decision_model: DecisionModelSettings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub limits: Option<LintLimits>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub filters: BTreeMap<FilterName, FilterWire>,
    pub rulebooks: Vec<CapturedRulebook>,
    pub evidence: CapturedEvidence,
}

/// A loaded configuration and the receipt of what `load` read.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct LoadedConfiguration {
    pub configuration: ResolvedConfiguration,
    /// Informational receipt only; never executable input.
    pub sources: Vec<DefinitionSnapshot>,
}

/// Closed diagnostic codes: existing CLI codes plus definition-acquisition codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum DiagnosticCode {
    NoConfigFound,
    Syntax,
    UnknownField,
    DuplicateKey,
    DuplicateEntry,
    SameRuleSetTwice,
    UnknownSelector,
    BadGlob,
    MissingRuleSet,
    UnsupportedInputs,
    OldSeverityWord,
    PathNotFound,
    NoGitForDefaultScope,
    BackendMissing,
    CredentialMissing,
    SourceNotFound,
    SourceUnreadable,
    SourceIo,
    SourceEncoding,
    CaptureMetadataInvalid,
}

impl From<LintInvalidCode> for DiagnosticCode {
    fn from(code: LintInvalidCode) -> Self {
        match code {
            LintInvalidCode::NoConfigFound => Self::NoConfigFound,
            LintInvalidCode::Syntax => Self::Syntax,
            LintInvalidCode::UnknownField => Self::UnknownField,
            LintInvalidCode::DuplicateKey => Self::DuplicateKey,
            LintInvalidCode::DuplicateEntry => Self::DuplicateEntry,
            LintInvalidCode::SameRuleSetTwice => Self::SameRuleSetTwice,
            LintInvalidCode::UnknownSelector => Self::UnknownSelector,
            LintInvalidCode::BadGlob => Self::BadGlob,
            LintInvalidCode::MissingRuleSet => Self::MissingRuleSet,
            LintInvalidCode::UnsupportedInputs => Self::UnsupportedInputs,
            LintInvalidCode::OldSeverityWord => Self::OldSeverityWord,
            LintInvalidCode::PathNotFound => Self::PathNotFound,
            LintInvalidCode::NoGitForDefaultScope => Self::NoGitForDefaultScope,
            LintInvalidCode::BackendMissing => Self::BackendMissing,
            LintInvalidCode::CredentialMissing => Self::CredentialMissing,
        }
    }
}

/// A refusal: which definition it concerns, and separately which checked input stopped it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub origin: Option<DefinitionOrigin>,
    /// The checked input that stopped execution, distinct from the definition origin.
    pub path: Option<PathBuf>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub json_path: Vec<String>,
    pub message: String,
}

/// Counts of an admitted configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct CheckSummary {
    pub rulebook_count: u32,
    pub active_rule_count: u32,
    pub skipped_rule_count: u32,
    pub evidence_kind_count: u32,
    pub filter_count: u32,
}

/// The native contract this addon implements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct ContractInfo {
    pub sdk_release: String,
    pub contract_revision: u32,
    pub target: String,
}

/// Where to discover and load an existing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct LoadOptions {
    /// Absolute directory to discover `sunstack.lint.json` from, as the CLI does from CWD.
    pub directory: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub config_path: Option<PathBuf>,
}

/// Which changes a run checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum Scope {
    Default,
    All,
    Since { rev: String },
}

/// The data part of a run request; cancellation is passed separately.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct RunRequest {
    /// Absolute checked root.
    pub root: PathBuf,
    pub scope: Scope,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "typescript", specta(optional))]
    pub paths: Vec<PathBuf>,
}

/// The literal `none` effects of a failure before dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum NoEffects {
    None,
}

/// The literal `unknown` effects of a failure after dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum UnknownEffects {
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum BeforeDispatchReason {
    UnsupportedRuntime,
    UnsupportedPlatform,
    AddonMissing,
    ContractMismatch,
    EncodingRejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum AfterDispatchReason {
    NativeCallFailed,
    NativeReplyInvalid,
}

/// A detectable adapter failure; effects are none before dispatch and unknown after.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum SdkFailure {
    BeforeDispatch {
        effects: NoEffects,
        reason: BeforeDispatchReason,
        message: String,
    },
    AfterDispatch {
        effects: UnknownEffects,
        reason: AfterDispatchReason,
        message: String,
    },
}

impl SdkFailure {
    pub fn after_dispatch(reason: AfterDispatchReason, message: impl Into<String>) -> Self {
        Self::AfterDispatch {
            effects: UnknownEffects::Unknown,
            reason,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[allow(clippy::large_enum_variant)]
pub enum LoadOutcome {
    Loaded { value: LoadedConfiguration },
    Invalid { diagnostics: Vec<Diagnostic> },
    Unavailable { diagnostics: Vec<Diagnostic> },
    Failed { failure: SdkFailure },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[allow(clippy::large_enum_variant)]
pub enum CheckOutcome {
    Valid {
        summary: CheckSummary,
        sources: Vec<DefinitionSnapshot>,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
    Unavailable {
        diagnostics: Vec<Diagnostic>,
    },
    Cancelled {
        diagnostics: Vec<Diagnostic>,
    },
    Failed {
        failure: SdkFailure,
    },
}
