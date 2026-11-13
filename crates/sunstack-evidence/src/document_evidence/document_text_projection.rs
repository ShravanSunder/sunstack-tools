use crate::FieldUnavailable;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DocumentText {
    #[default]
    Raw,
    HtmlText,
    JsonString,
}

impl DocumentText {
    pub(crate) fn is_raw(&self) -> bool {
        *self == Self::Raw
    }

    pub(crate) fn project(self, raw: &str) -> Result<String, FieldUnavailable> {
        match self {
            Self::Raw => Ok(raw.to_owned()),
            Self::HtmlText => super::html_decoded_text::decoded_html_text(raw),
            Self::JsonString => {
                serde_json::from_str::<String>(raw).map_err(|_| FieldUnavailable::NotAString)
            }
        }
    }
}

#[cfg(test)]
#[path = "document_text_projection_test.rs"]
mod tests;
