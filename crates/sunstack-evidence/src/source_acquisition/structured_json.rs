use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Recursive JSON content. This is data, never a declaration or an executable expression.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum StructuredJson {
    Null,
    Boolean(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<StructuredJson>),
    Object(BTreeMap<String, StructuredJson>),
}
impl StructuredJson {
    pub fn pointer(&self, pointer: &str) -> Option<&Self> {
        if pointer.is_empty() {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        let mut current = self;
        for token in pointer[1..].split('/') {
            let mut decoded = String::new();
            let mut chars = token.chars();
            while let Some(character) = chars.next() {
                if character == '~' {
                    decoded.push(match chars.next()? {
                        '0' => '~',
                        '1' => '/',
                        _ => return None,
                    });
                } else {
                    decoded.push(character);
                }
            }
            current = match current {
                Self::Object(fields) => fields.get(&decoded)?,
                Self::Array(elements) => {
                    if decoded.is_empty()
                        || !decoded.bytes().all(|byte| byte.is_ascii_digit())
                        || (decoded.len() > 1 && decoded.starts_with('0'))
                    {
                        return None;
                    }
                    elements.get(decoded.parse::<usize>().ok()?)?
                }
                _ => return None,
            };
        }
        Some(current)
    }
}

#[cfg(test)]
#[path = "structured_json_test.rs"]
mod tests;
