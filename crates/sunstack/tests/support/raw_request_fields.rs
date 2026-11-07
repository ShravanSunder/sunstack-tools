// Compare wire values without reparsing and rewriting their key/escape/number order.
pub fn compact_json_value(raw: &str) -> Vec<u8> {
    let mut compact = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    for byte in raw.bytes() {
        if quoted {
            compact.push(byte);
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else if byte == b'"' {
            quoted = true;
            compact.push(byte);
        } else if !byte.is_ascii_whitespace() {
            compact.push(byte);
        }
    }
    compact
}

#[derive(serde::Deserialize)]
pub struct RawRequestFields<'a> {
    #[serde(borrow)]
    pub state: &'a serde_json::value::RawValue,
    #[serde(borrow)]
    pub questions: &'a serde_json::value::RawValue,
}
