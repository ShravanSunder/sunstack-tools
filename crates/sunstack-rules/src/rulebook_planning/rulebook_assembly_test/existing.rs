#![allow(clippy::unwrap_used)]
use super::super::*;
#[test]
fn raw_entry_decoder_preserves_duplicates_so_assembly_can_refuse_the_marker() {
    let raw: RawEntries<'_> =
        serde_json::from_str(r#"{"K001":{"name":"first"},"K001":{"name":"second"}}"#).unwrap();
    assert_eq!(raw.0.len(), 2);
    assert_eq!(raw.0[0].0, raw.0[1].0);
    assert_ne!(raw.0[0].1.get(), raw.0[1].1.get());
}
