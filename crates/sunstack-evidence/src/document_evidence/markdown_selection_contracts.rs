use crate::SourceSpan;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownUnitKind {
    Section,
    Paragraph,
    ListItem,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum MarkdownUnitFilter {
    StartsWith { starts_with: String },
    Regex { pattern: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(from = "SelectionWire", into = "SelectionWire")]
pub enum MarkdownUnitSelection {
    Unit {
        units: Vec<MarkdownUnitKind>,
        filter: Option<MarkdownUnitFilter>,
    },
    CallSite {
        units: Vec<MarkdownUnitKind>,
        filter: MarkdownUnitFilter,
    },
}

// Flat wire alternatives are strict individually; nested Serde flatten would
// reject valid filter fields together with deny_unknown_fields.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
enum SelectionWire {
    Unfiltered {
        mode: UnitOnly,
        #[schemars(length(min = 1))]
        units: Vec<MarkdownUnitKind>,
    },
    StartsWith {
        mode: SelectionMode,
        #[schemars(length(min = 1))]
        units: Vec<MarkdownUnitKind>,
        #[schemars(length(min = 1))]
        starts_with: String,
    },
    Regex {
        mode: SelectionMode,
        #[schemars(length(min = 1))]
        units: Vec<MarkdownUnitKind>,
        #[schemars(length(min = 1))]
        pattern: String,
    },
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UnitOnly {
    Unit,
}

/// The `markdown_units` selection a rulebook evidence kind accepts: the flat wire of
/// [`MarkdownUnitSelection`] in unit mode only, as the kind schema restricts it. Declares
/// the kind boundary for TypeScript; values still decode as [`MarkdownUnitSelection`].
#[cfg(feature = "typescript")]
#[derive(specta::Type, Serialize)]
#[serde(untagged)]
#[allow(
    dead_code,
    reason = "declaration-only for TypeScript generation; built only by its decode agreement test"
)]
pub(crate) enum MarkdownUnitsKindWire {
    Unfiltered {
        mode: UnitOnly,
        units: Vec<MarkdownUnitKind>,
    },
    StartsWith {
        mode: UnitOnly,
        units: Vec<MarkdownUnitKind>,
        starts_with: String,
    },
    Regex {
        mode: UnitOnly,
        units: Vec<MarkdownUnitKind>,
        pattern: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum SelectionMode {
    Unit,
    CallSite,
}

impl From<SelectionWire> for MarkdownUnitSelection {
    fn from(wire: SelectionWire) -> Self {
        let (mode, units, filter) = match wire {
            SelectionWire::Unfiltered { units, .. } => {
                return Self::Unit {
                    units,
                    filter: None,
                };
            }
            SelectionWire::StartsWith {
                mode,
                units,
                starts_with,
            } => (mode, units, MarkdownUnitFilter::StartsWith { starts_with }),
            SelectionWire::Regex {
                mode,
                units,
                pattern,
            } => (mode, units, MarkdownUnitFilter::Regex { pattern }),
        };
        match mode {
            SelectionMode::Unit => Self::Unit {
                units,
                filter: Some(filter),
            },
            SelectionMode::CallSite => Self::CallSite { units, filter },
        }
    }
}

impl From<MarkdownUnitSelection> for SelectionWire {
    fn from(selection: MarkdownUnitSelection) -> Self {
        let (mode, units, filter) = match selection {
            MarkdownUnitSelection::Unit {
                units,
                filter: None,
            } => {
                return Self::Unfiltered {
                    mode: UnitOnly::Unit,
                    units,
                };
            }
            MarkdownUnitSelection::Unit {
                units,
                filter: Some(filter),
            } => (SelectionMode::Unit, units, filter),
            MarkdownUnitSelection::CallSite { units, filter } => {
                (SelectionMode::CallSite, units, filter)
            }
        };
        match filter {
            MarkdownUnitFilter::StartsWith { starts_with } => Self::StartsWith {
                mode,
                units,
                starts_with,
            },
            MarkdownUnitFilter::Regex { pattern } => Self::Regex {
                mode,
                units,
                pattern,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MarkdownByteRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MarkdownOccurrence {
    Unit {
        span: SourceSpan,
    },
    CallSite {
        unit_span: SourceSpan,
        match_span: SourceSpan,
        visible_range: MarkdownByteRange,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectedMarkdownUnit {
    pub kind: MarkdownUnitKind,
    pub span: SourceSpan,
    pub text: String,
    pub visible_text: String,
    pub occurrence: MarkdownOccurrence,
}

#[cfg(all(test, feature = "typescript"))]
#[path = "markdown_selection_contracts_kind_wire_test.rs"]
mod kind_wire_tests;
