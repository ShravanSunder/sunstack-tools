use crate::LintReport;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use sunstack_decisions::ChoiceLabel;
use sunstack_rules::{FilterName, RuleId, StepName};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaseFile {
    pub repository: Option<PathBuf>,
    pub cases: Vec<LintCase>,
    #[serde(default)]
    pub bars: Vec<RecordedBar>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LabelSource {
    OwnerFix,
    OwnerRuling,
    BlindAgents,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedRuleOutcome {
    Warn,
    Pass,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CaseSubject {
    Rule {
        rule: RuleId,
        file: PathBuf,
        lines: [usize; 2],
        text_sha256: String,
        expected: ExpectedRuleOutcome,
    },
    Filter {
        filter: FilterName,
        folder: PathBuf,
        reads_sha256: String,
        expected_label: ChoiceLabel,
    },
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LintCase {
    pub id: String,
    pub revision: String,
    pub set: String,
    pub label_from: LabelSource,
    #[serde(flatten)]
    pub subject: CaseSubject,
}
impl LintCase {
    pub(super) fn binding(&self) -> crate::CaseBinding {
        let subject = match &self.subject {
            CaseSubject::Rule {
                rule,
                file,
                lines,
                text_sha256,
                ..
            } => crate::CaseSubjectBinding::Rule {
                rule: rule.clone(),
                file: file.clone(),
                lines: *lines,
                text_sha256: text_sha256.clone(),
            },
            CaseSubject::Filter {
                filter,
                folder,
                reads_sha256,
                ..
            } => crate::CaseSubjectBinding::Filter {
                filter: filter.clone(),
                folder: folder.clone(),
                reads_sha256: reads_sha256.clone(),
            },
        };
        crate::CaseBinding {
            id: self.id.clone(),
            subject,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BarBound {
    YesAtOrAbove,
    NoAtOrBelow,
    MinProbability,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedBar {
    #[serde(rename = "for")]
    pub owner: String,
    pub step: StepName,
    pub bound: BarBound,
    pub value: f64,
    pub set_from: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarState {
    Assessed,
    Unassessed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarAssessment {
    #[serde(rename = "for")]
    pub owner: String,
    pub step: StepName,
    pub bound: BarBound,
    pub value: Option<f64>,
    pub set_from: Vec<String>,
    pub current: Option<f64>,
    pub state: BarState,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarCrossing {
    pub step: StepName,
    pub bound: BarBound,
    pub bar: f64,
    pub observed: Option<f64>,
    pub crossed: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CaseOutcome {
    Prepared,
    Right,
    FalseWarning,
    Missed,
    WrongLabel,
    Unsure,
    Incomplete {
        reason: String,
    },
    NotJudged {
        reason: String,
    },
    Stale,
    NoMatchingPiece,
    MoreThanOneMatchingPiece,
    FilterExcluded,
    SelectionPending {
        evidence_kind: sunstack_evidence::EvidenceKindName,
        file: PathBuf,
    },
    Cancelled,
}
impl CaseOutcome {
    pub(super) fn key(&self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::Right => "right",
            Self::FalseWarning => "false_warning",
            Self::Missed => "missed",
            Self::WrongLabel => "wrong_label",
            Self::Unsure => "unsure",
            Self::Incomplete { .. } => "incomplete",
            Self::NotJudged { .. } => "not_judged",
            Self::Stale => "stale",
            Self::NoMatchingPiece => "no_matching_piece",
            Self::MoreThanOneMatchingPiece => "more_than_one_matching_piece",
            Self::FilterExcluded => "filter_excluded",
            Self::SelectionPending { .. } => "selection_pending",
            Self::Cancelled => "cancelled",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseResult {
    pub id: String,
    #[serde(rename = "case")]
    pub definition: LintCase,
    pub outcome: CaseOutcome,
    pub result_id: Option<usize>,
    pub bars: Vec<BarCrossing>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionRun {
    pub revision: String,
    pub resolved_revision: String,
    pub report: LintReport,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasesReport {
    #[serde(default, flatten)]
    pub preparation: Option<PreparedCasesReport>,
    pub errors: BTreeMap<String, String>,
    pub not_selected: Vec<String>,
    pub by_rule: BTreeMap<RuleId, BTreeMap<String, usize>>,
    pub by_filter: BTreeMap<FilterName, BTreeMap<String, usize>>,
    pub cases: Vec<CaseResult>,
    pub counts: BTreeMap<String, usize>,
    pub bars: Vec<BarAssessment>,
    pub runs: Vec<RevisionRun>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedCasesReport {
    pub mode: crate::PrepareMode,
    pub status: crate::LintStatus,
}
impl CasesReport {
    pub fn exit_code(&self) -> u8 {
        if let Some(preparation) = &self.preparation {
            return preparation.status.exit_code();
        }
        if self
            .cases
            .iter()
            .any(|case| matches!(case.outcome, CaseOutcome::Cancelled))
        {
            return 4;
        }
        if self
            .cases
            .iter()
            .all(|case| matches!(case.outcome, CaseOutcome::Right))
            && self
                .bars
                .iter()
                .all(|bar| matches!(bar.state, BarState::Assessed))
        {
            0
        } else {
            1
        }
    }
}
