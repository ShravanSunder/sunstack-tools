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

/// Whether a value supplied back through the SDK's JSON transport is the original value.
///
/// JavaScript numbers do not keep the spelling of integral floats, so an unchanged `2.0`
/// returns as `2`. Such a pair is the same value; every other difference, including
/// integers that differ only beyond f64 precision, is a change. Equality never rounds an
/// integer through f64.
pub fn same_after_transport(supplied: &serde_json::Value, original: &serde_json::Value) -> bool {
    use serde_json::Value;
    match (supplied, original) {
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, item)| {
                    right
                        .get(key)
                        .is_some_and(|other| same_after_transport(item, other))
                })
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(item, other)| same_after_transport(item, other))
        }
        (Value::Number(left), Value::Number(right)) => same_number(left, right),
        _ => supplied == original,
    }
}

/// Equal numbers, where an integer and an integral float are equal only if the integer
/// converts to that float exactly.
fn same_number(left: &serde_json::Number, right: &serde_json::Number) -> bool {
    let exact_integer = |number: &serde_json::Number| {
        number
            .as_u64()
            .map(i128::from)
            .or_else(|| number.as_i64().map(i128::from))
    };
    match (exact_integer(left), exact_integer(right)) {
        (Some(left), Some(right)) => left == right,
        (None, None) => left.as_f64() == right.as_f64(),
        (Some(integer), None) => integral_float_is(right.as_f64(), integer),
        (None, Some(integer)) => integral_float_is(left.as_f64(), integer),
    }
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn integral_float_is(float: Option<f64>, integer: i128) -> bool {
    let converted = integer as f64;
    float.is_some_and(|float| float == converted) && converted as i128 == integer
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
