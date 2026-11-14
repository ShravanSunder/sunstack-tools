use crate::{EvidenceCode, EvidenceError, EvidenceResult, HeadingLevels};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HtmlUnitKind {
    Section,
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum HtmlUnitFilter {
    StartsWith { starts_with: String },
    Regex { pattern: String },
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "SelectionWire", into = "SelectionWire")]
#[schemars(with = "SelectionWire")]
pub enum HtmlUnitSelection {
    Unit {
        units: Vec<HtmlUnitKind>,
        filter: Option<HtmlUnitFilter>,
        levels: Option<HeadingLevels>,
    },
}

// Each flat alternative closes its filter keys without flattening an untagged enum.
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
#[schemars(extend("allOf" = [{"if":{"required":["levels"]},"then":{"properties":{"units":{"const":["section"]}}}}]))]
enum SelectionWire {
    Unfiltered {
        mode: UnitOnly,
        #[schemars(length(min = 1))]
        units: Vec<HtmlUnitKind>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "super::heading_levels::optional_heading_levels"
        )]
        #[schemars(with = "HeadingLevels")]
        #[cfg_attr(feature = "typescript", specta(type = HeadingLevels, optional))]
        levels: Option<HeadingLevels>,
    },
    StartsWith {
        mode: UnitOnly,
        #[schemars(length(min = 1))]
        units: Vec<HtmlUnitKind>,
        #[schemars(length(min = 1))]
        starts_with: String,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "super::heading_levels::optional_heading_levels"
        )]
        #[schemars(with = "HeadingLevels")]
        #[cfg_attr(feature = "typescript", specta(type = HeadingLevels, optional))]
        levels: Option<HeadingLevels>,
    },
    Regex {
        mode: UnitOnly,
        #[schemars(length(min = 1))]
        units: Vec<HtmlUnitKind>,
        #[schemars(length(min = 1))]
        pattern: String,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "super::heading_levels::optional_heading_levels"
        )]
        #[schemars(with = "HeadingLevels")]
        #[cfg_attr(feature = "typescript", specta(type = HeadingLevels, optional))]
        levels: Option<HeadingLevels>,
    },
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum UnitOnly {
    Unit,
}

impl TryFrom<SelectionWire> for HtmlUnitSelection {
    type Error = EvidenceError;

    fn try_from(wire: SelectionWire) -> Result<Self, Self::Error> {
        let (units, filter, levels) = match wire {
            SelectionWire::Unfiltered { units, levels, .. } => (units, None, levels),
            SelectionWire::StartsWith {
                units,
                starts_with,
                levels,
                ..
            } => (
                units,
                Some(HtmlUnitFilter::StartsWith { starts_with }),
                levels,
            ),
            SelectionWire::Regex {
                units,
                pattern,
                levels,
                ..
            } => (units, Some(HtmlUnitFilter::Regex { pattern }), levels),
        };
        let selection = Self::Unit {
            units,
            filter,
            levels,
        };
        selection.validate()?;
        Ok(selection)
    }
}

impl From<HtmlUnitSelection> for SelectionWire {
    fn from(selection: HtmlUnitSelection) -> Self {
        let HtmlUnitSelection::Unit {
            units,
            filter,
            levels,
        } = selection;
        match filter {
            None => Self::Unfiltered {
                mode: UnitOnly::Unit,
                units,
                levels,
            },
            Some(HtmlUnitFilter::StartsWith { starts_with }) => Self::StartsWith {
                mode: UnitOnly::Unit,
                units,
                starts_with,
                levels,
            },
            Some(HtmlUnitFilter::Regex { pattern }) => Self::Regex {
                mode: UnitOnly::Unit,
                units,
                pattern,
                levels,
            },
        }
    }
}

impl HtmlUnitSelection {
    pub fn validate(&self) -> EvidenceResult<()> {
        let Self::Unit {
            units,
            filter,
            levels,
        } = self;
        if units.is_empty() || (levels.is_some() && units.as_slice() != [HtmlUnitKind::Section]) {
            return Err(EvidenceCode::InvalidSelector.into());
        }
        match filter {
            Some(HtmlUnitFilter::StartsWith { starts_with }) if starts_with.is_empty() => {
                return Err(EvidenceCode::InvalidSelector.into());
            }
            Some(HtmlUnitFilter::Regex { pattern }) => {
                regex::Regex::new(pattern).map_err(|_| EvidenceCode::InvalidSelector)?;
                let expression = regex_syntax::Parser::new()
                    .parse(pattern)
                    .map_err(|_| EvidenceCode::InvalidSelector)?;
                if expression.properties().minimum_len() == Some(0) {
                    return Err(EvidenceCode::InvalidSelector.into());
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn matches_text(&self, text: &str) -> EvidenceResult<bool> {
        self.validate()?;
        let Self::Unit { filter, .. } = self;
        match filter {
            None => Ok(true),
            Some(HtmlUnitFilter::StartsWith { starts_with }) => Ok(text.starts_with(starts_with)),
            Some(HtmlUnitFilter::Regex { pattern }) => regex::Regex::new(pattern)
                .map(|expression| expression.is_match(text))
                .map_err(|_| EvidenceCode::InvalidSelector.into()),
        }
    }

    pub fn matches_heading_level(&self, level: u8) -> bool {
        let Self::Unit { levels, .. } = self;
        levels.as_ref().is_none_or(|levels| levels.contains(level))
    }
}

#[cfg(test)]
#[path = "html_selection_contracts_test.rs"]
mod tests;
