//! The Rust-normalized serialization used for inline and edited definition digests.
//!
//! Values are first serialized by their typed Rust contracts, which decide which
//! optional fields are omitted. Normalization then only sorts object keys
//! recursively: arrays keep authored order, explicit `null` data is kept, integers
//! keep their exact decimal tokens and floats use the pinned `serde_json` encoder.

/// Recursively sort object keys; every value, including explicit null, is kept.
pub fn canonical_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(object) => {
            let mut entries = object
                .into_iter()
                .map(|(key, item)| (key, canonical_value(item)))
                .collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            serde_json::Value::Object(entries.into_iter().collect())
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(canonical_value).collect())
        }
        other => other,
    }
}

/// Normalized UTF-8 text of a value, as hashed into snapshots.
pub fn normalized_text(value: &serde_json::Value) -> String {
    canonical_value(value.clone()).to_string()
}

/// One escaped JSON Pointer reference token (RFC 6901).
pub fn pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

/// A deterministic inline source ID: configuration ID plus a JSON Pointer.
pub fn source_id(configuration_id: &str, tokens: &[&str]) -> String {
    let pointer = tokens
        .iter()
        .map(|token| format!("/{}", pointer_token(token)))
        .collect::<String>();
    format!("{configuration_id}#{pointer}")
}

#[cfg(test)]
#[path = "normalized_values_test.rs"]
mod tests;
