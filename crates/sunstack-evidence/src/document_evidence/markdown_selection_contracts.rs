use crate::{EvidenceCode, EvidenceError, HeadingLevels, SourceSpan};
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
#[serde(try_from = "SelectionWire", into = "SelectionWire")]
#[schemars(with = "SelectionWire")]
pub enum MarkdownUnitSelection {
    Unit {
        units: Vec<MarkdownUnitKind>,
        filter: Option<MarkdownUnitFilter>,
        levels: Option<HeadingLevels>,
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
#[schemars(extend("allOf" = [{"if":{"required":["levels"]},"then":{"properties":{"mode":{"const":"unit"},"units":{"const":["section"]}}}}]))]
enum SelectionWire {
    Unfiltered {
        mode: UnitOnly,
        #[schemars(length(min = 1))]
        units: Vec<MarkdownUnitKind>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "super::heading_levels::optional_heading_levels"
        )]
        #[schemars(with = "HeadingLevels")]
        levels: Option<HeadingLevels>,
    },
    StartsWith {
        mode: SelectionMode,
        #[schemars(length(min = 1))]
        units: Vec<MarkdownUnitKind>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "super::heading_levels::optional_heading_levels"
        )]
        #[schemars(with = "HeadingLevels")]
        levels: Option<HeadingLevels>,
        #[schemars(length(min = 1))]
        starts_with: String,
    },
    Regex {
        mode: SelectionMode,
        #[schemars(length(min = 1))]
        units: Vec<MarkdownUnitKind>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "super::heading_levels::optional_heading_levels"
        )]
        #[schemars(with = "HeadingLevels")]
        levels: Option<HeadingLevels>,
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
        #[serde(skip_serializing_if = "Option::is_none")]
        #[specta(type = HeadingLevels, optional)]
        levels: Option<HeadingLevels>,
    },
    StartsWith {
        mode: UnitOnly,
        units: Vec<MarkdownUnitKind>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[specta(type = HeadingLevels, optional)]
        levels: Option<HeadingLevels>,
        starts_with: String,
    },
    Regex {
        mode: UnitOnly,
        units: Vec<MarkdownUnitKind>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[specta(type = HeadingLevels, optional)]
        levels: Option<HeadingLevels>,
        pattern: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum SelectionMode {
    Unit,
    CallSite,
}

impl TryFrom<SelectionWire> for MarkdownUnitSelection {
    type Error = EvidenceError;

    fn try_from(wire: SelectionWire) -> Result<Self, Self::Error> {
        let (mode, units, filter, levels) = match wire {
            SelectionWire::Unfiltered { units, levels, .. } => {
                (SelectionMode::Unit, units, None, levels)
            }
            SelectionWire::StartsWith {
                mode,
                units,
                starts_with,
                levels,
            } => (
                mode,
                units,
                Some(MarkdownUnitFilter::StartsWith { starts_with }),
                levels,
            ),
            SelectionWire::Regex {
                mode,
                units,
                pattern,
                levels,
            } => (
                mode,
                units,
                Some(MarkdownUnitFilter::Regex { pattern }),
                levels,
            ),
        };
        if levels.is_some()
            && (!matches!(mode, SelectionMode::Unit)
                || units.as_slice() != [MarkdownUnitKind::Section])
        {
            return Err(EvidenceCode::InvalidSelector.into());
        }
        Ok(match mode {
            SelectionMode::Unit => Self::Unit {
                units,
                filter,
                levels,
            },
            SelectionMode::CallSite => Self::CallSite {
                units,
                filter: filter.ok_or(EvidenceCode::InvalidSelector)?,
            },
        })
    }
}

impl From<MarkdownUnitSelection> for SelectionWire {
    fn from(selection: MarkdownUnitSelection) -> Self {
        let (mode, units, filter, levels) = match selection {
            MarkdownUnitSelection::Unit {
                units,
                filter: None,
                levels,
            } => {
                return Self::Unfiltered {
                    mode: UnitOnly::Unit,
                    units,
                    levels,
                };
            }
            MarkdownUnitSelection::Unit {
                units,
                filter: Some(filter),
                levels,
            } => (SelectionMode::Unit, units, filter, levels),
            MarkdownUnitSelection::CallSite { units, filter } => {
                (SelectionMode::CallSite, units, filter, None)
            }
        };
        match filter {
            MarkdownUnitFilter::StartsWith { starts_with } => Self::StartsWith {
                mode,
                units,
                starts_with,
                levels,
            },
            MarkdownUnitFilter::Regex { pattern } => Self::Regex {
                mode,
                units,
                pattern,
                levels,
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
