use crate::{EvidenceCode, EvidenceError, EvidenceResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HtmlUnitKind {
    Section,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum HtmlUnitFilter {
    StartsWith { starts_with: String },
    Regex { pattern: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "SelectionWire", into = "SelectionWire")]
#[schemars(with = "SelectionWire")]
pub enum HtmlUnitSelection {
    Unit {
        units: Vec<HtmlUnitKind>,
        filter: Option<HtmlUnitFilter>,
    },
}

// Each flat alternative closes its filter keys without flattening an untagged enum.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
enum SelectionWire {
    Unfiltered {
        mode: UnitOnly,
        #[schemars(length(min = 1))]
        units: Vec<HtmlUnitKind>,
    },
    StartsWith {
        mode: UnitOnly,
        #[schemars(length(min = 1))]
        units: Vec<HtmlUnitKind>,
        #[schemars(length(min = 1))]
        starts_with: String,
    },
    Regex {
        mode: UnitOnly,
        #[schemars(length(min = 1))]
        units: Vec<HtmlUnitKind>,
        #[schemars(length(min = 1))]
        pattern: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum UnitOnly {
    Unit,
}

impl TryFrom<SelectionWire> for HtmlUnitSelection {
    type Error = EvidenceError;

    fn try_from(wire: SelectionWire) -> Result<Self, Self::Error> {
        let (units, filter) = match wire {
            SelectionWire::Unfiltered { units, .. } => (units, None),
            SelectionWire::StartsWith {
                units, starts_with, ..
            } => (units, Some(HtmlUnitFilter::StartsWith { starts_with })),
            SelectionWire::Regex { units, pattern, .. } => {
                (units, Some(HtmlUnitFilter::Regex { pattern }))
            }
        };
        let selection = Self::Unit { units, filter };
        selection.validate()?;
        Ok(selection)
    }
}

impl From<HtmlUnitSelection> for SelectionWire {
    fn from(selection: HtmlUnitSelection) -> Self {
        let HtmlUnitSelection::Unit { units, filter } = selection;
        match filter {
            None => Self::Unfiltered {
                mode: UnitOnly::Unit,
                units,
            },
            Some(HtmlUnitFilter::StartsWith { starts_with }) => Self::StartsWith {
                mode: UnitOnly::Unit,
                units,
                starts_with,
            },
            Some(HtmlUnitFilter::Regex { pattern }) => Self::Regex {
                mode: UnitOnly::Unit,
                units,
                pattern,
            },
        }
    }
}

impl HtmlUnitSelection {
    pub fn validate(&self) -> EvidenceResult<()> {
        let Self::Unit { units, filter } = self;
        if units.is_empty() {
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
}

#[cfg(test)]
#[path = "html_selection_contracts_test.rs"]
mod tests;
