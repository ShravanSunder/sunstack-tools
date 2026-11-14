use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0} must be nonempty")]
pub struct InvalidHtmlName(&'static str);

macro_rules! html_name {
    ($name:ident, $label:literal) => {
        #[cfg_attr(feature = "typescript", derive(specta::Type))]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, InvalidHtmlName> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(InvalidHtmlName($label));
                }
                Ok(Self(value))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl TryFrom<String> for $name {
            type Error = InvalidHtmlName;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
        impl JsonSchema for $name {
            fn schema_name() -> std::borrow::Cow<'static, str> {
                stringify!($name).into()
            }
            fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
                schemars::json_schema!({"type":"string","minLength":1,"pattern":"\\S"})
            }
        }
    };
}

html_name!(HtmlTagName, "HTML tag name");
html_name!(HtmlAttributeName, "HTML attribute name");

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct HtmlEvidenceSettings {
    pub skip_tags: Vec<HtmlTagName>,
    pub text_attributes: BTreeMap<HtmlTagName, Vec<HtmlAttributeName>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub embedded_markdown: Vec<HtmlMarkdownIsland>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HtmlMarkdownIsland {
    pub tag: HtmlTagName,
    #[serde(default)]
    pub attributes: BTreeMap<HtmlAttributeName, String>,
}

impl HtmlEvidenceSettings {
    pub fn is_default(&self) -> bool {
        self.skip_tags.is_empty()
            && self.text_attributes.is_empty()
            && self.embedded_markdown.is_empty()
    }
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct MarkdownEvidenceSettings {
    pub heading_anchors: Option<crate::HeadingAnchorScheme>,
}

impl MarkdownEvidenceSettings {
    pub fn is_default(&self) -> bool {
        self.heading_anchors.is_none()
    }
}

/// Immutable project conventions shared by direct evidence APIs and repository lint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct DocumentEvidenceSettings {
    pub html: HtmlEvidenceSettings,
    pub markdown: MarkdownEvidenceSettings,
}
