use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "Vec<u8>", into = "Vec<u8>")]
#[schemars(extend("minItems" = 1, "uniqueItems" = true, "items" = {"type":"integer","minimum":1,"maximum":6}))]
pub struct HeadingLevels(Vec<u8>);

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum InvalidHeadingLevels {
    #[error("levels must contain at least one heading level")]
    Empty,
    #[error("heading level {0} is outside 1..6")]
    OutOfRange(u8),
    #[error("levels repeats heading level {0}")]
    Repeated(u8),
}

impl HeadingLevels {
    pub fn new(levels: Vec<u8>) -> Result<Self, InvalidHeadingLevels> {
        if levels.is_empty() {
            return Err(InvalidHeadingLevels::Empty);
        }
        let mut seen = std::collections::BTreeSet::new();
        for level in &levels {
            if !(1..=6).contains(level) {
                return Err(InvalidHeadingLevels::OutOfRange(*level));
            }
            if !seen.insert(*level) {
                return Err(InvalidHeadingLevels::Repeated(*level));
            }
        }
        Ok(Self(levels))
    }

    pub fn contains(&self, level: u8) -> bool {
        self.0.contains(&level)
    }
}

impl TryFrom<Vec<u8>> for HeadingLevels {
    type Error = InvalidHeadingLevels;

    fn try_from(levels: Vec<u8>) -> Result<Self, Self::Error> {
        Self::new(levels)
    }
}

impl From<HeadingLevels> for Vec<u8> {
    fn from(levels: HeadingLevels) -> Self {
        levels.0
    }
}

pub(crate) fn optional_heading_levels<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<HeadingLevels>, D::Error> {
    HeadingLevels::deserialize(deserializer).map(Some)
}

#[cfg(test)]
#[path = "heading_levels_test.rs"]
mod tests;
