use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use sunstack_evidence::{SourceInput, SourceName, SourceOccurrence};
use sunstack_rules::{EvidenceBinding, QuestionRef, RuleRef};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CaseSource {
    pub source_id: CaseSourceId,
    pub source: SourceInput,
    pub evidence: BTreeMap<SourceName, EvidenceBinding>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Label {
    Yes,
    No,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Split {
    Development,
    HeldOut,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LabeledCase {
    pub case_id: CaseId,
    pub source: CaseSourceId,
    pub project: Option<ProjectKey>,
    pub source_anchor: SourceOccurrence,
    pub question: QuestionRef,
    pub label: Label,
    pub split: Split,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FreezeInput {
    pub definition_root: PathBuf,
    pub rule: RuleRef,
    pub split: Split,
    pub sources: Vec<CaseSource>,
    pub labels: Vec<LabeledCase>,
    pub source_files: Vec<PathBuf>,
    pub definition_files: Vec<PathBuf>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FrozenFile {
    pub path: PathBuf,
    pub content_identity: ContentIdentity,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CaseSetManifest {
    pub rule: RuleRef,
    pub split: Split,
    pub sources: FrozenFile,
    pub labels: FrozenFile,
    pub source_files: Vec<FrozenFile>,
    pub definition_files: Vec<FrozenFile>,
}
/// A verified, immutable snapshot; construct it by freezing or loading a manifest.
#[derive(Clone, Debug)]
pub struct FrozenCaseSet {
    pub(crate) manifest: CaseSetManifest,
    pub(crate) identity: CaseSetIdentity,
    pub(crate) rule_version: ContentIdentity,
    pub(crate) definition_home: PathBuf,
    pub(crate) pass_means: sunstack_rules::PassMeans,
    pub(crate) sources: Vec<CaseSource>,
    pub(crate) labels: Vec<LabeledCase>,
}
impl FrozenCaseSet {
    /// The owned frozen definitions directory to use for replay.
    pub fn definition_root(&self) -> &std::path::Path {
        &self.definition_home
    }
    /// The absolute rule identity emitted by replaying these frozen definitions.
    pub fn replay_rule(&self) -> RuleRef {
        RuleRef {
            rule_set: self.definition_home.join(&self.manifest.rule.rule_set),
            rule_id: self.manifest.rule.rule_id.clone(),
        }
    }

    pub fn pass_means(&self) -> sunstack_rules::PassMeans {
        self.pass_means
    }

    pub fn identity(&self) -> &CaseSetIdentity {
        &self.identity
    }
    pub fn rule_version(&self) -> &ContentIdentity {
        &self.rule_version
    }
    pub fn sources(&self) -> &[CaseSource] {
        &self.sources
    }
    pub fn manifest(&self) -> &CaseSetManifest {
        &self.manifest
    }
    pub fn labels(&self) -> &[LabeledCase] {
        &self.labels
    }
}
