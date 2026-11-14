#![allow(clippy::unwrap_used)]
use super::*;

/// Every kind-boundary declaration form serializes to a wire the real decoder reads as a
/// unit selection with the same units and filter.
#[test]
fn kind_wire_forms_decode_as_unit_selections() {
    let units = vec![MarkdownUnitKind::Section, MarkdownUnitKind::ListItem];
    let cases = [
        (
            MarkdownUnitsKindWire::Unfiltered {
                mode: UnitOnly::Unit,
                units: units.clone(),
            },
            None,
        ),
        (
            MarkdownUnitsKindWire::StartsWith {
                mode: UnitOnly::Unit,
                units: units.clone(),
                starts_with: "Steps".into(),
            },
            Some(MarkdownUnitFilter::StartsWith {
                starts_with: "Steps".into(),
            }),
        ),
        (
            MarkdownUnitsKindWire::Regex {
                mode: UnitOnly::Unit,
                units: units.clone(),
                pattern: "^S".into(),
            },
            Some(MarkdownUnitFilter::Regex {
                pattern: "^S".into(),
            }),
        ),
    ];
    for (declared, filter) in cases {
        let wire = serde_json::to_value(&declared).unwrap();
        let decoded: MarkdownUnitSelection = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(
            decoded,
            MarkdownUnitSelection::Unit {
                units: units.clone(),
                filter
            }
        );
        assert_eq!(serde_json::to_value(&decoded).unwrap(), wire);
    }
}
