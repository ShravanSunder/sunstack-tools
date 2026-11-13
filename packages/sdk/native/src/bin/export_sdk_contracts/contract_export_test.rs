#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use specta::datatype::Struct;

fn field(optional: bool) -> Field {
    let mut field = Field::new(DataType::Primitive(Primitive::str));
    field.optional = optional;
    field
}

/// An `EvidenceKindWire`-shaped struct with the given fields, each a string.
fn wire(fields: &[(&str, bool)]) -> Option<DataType> {
    let mut builder = Struct::named();
    for (name, optional) in fields {
        builder = builder.field(name.to_string(), field(*optional));
    }
    Some(builder.build())
}

const KNOWN: [(&str, bool); 8] = [
    ("rule", true),
    ("inline_rule", true),
    ("html_units", true),
    ("markdown_units", true),
    ("fields", false),
    ("max_chars", true),
    ("select", true),
    ("within", true),
];

#[test]
fn the_known_wire_becomes_shared_fields_and_four_exclusive_matchers() {
    let Ok(DataType::Intersection(members)) = exclusive_matcher(wire(&KNOWN)) else {
        panic!("the known wire is declared");
    };
    let [DataType::Struct(shared), DataType::Enum(choice)] = members.as_slice() else {
        panic!("shared fields intersected with one matcher choice");
    };
    let Fields::Named(shared) = &shared.fields else {
        panic!("named shared fields");
    };
    let names = shared
        .fields
        .iter()
        .map(|(name, _)| name.to_string())
        .collect::<Vec<_>>();
    assert_eq!(names, ["fields", "max_chars", "select", "within"]);
    let variants = choice
        .variants
        .iter()
        .map(|(name, _)| name.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        variants,
        ["Rule", "InlineRule", "HtmlUnits", "MarkdownUnits"]
    );
    assert!(
        choice
            .attributes
            .contains_key(specta::markers::SERDE_CONTAINER_UNTAGGED)
    );
}

#[test]
fn a_wire_that_no_longer_matches_the_known_fields_fails_generation() {
    let with = |extra: (&'static str, bool)| {
        let mut fields = KNOWN.to_vec();
        fields.push(extra);
        fields
    };
    let without = |name: &str| {
        KNOWN
            .iter()
            .copied()
            .filter(|(field, _)| *field != name)
            .collect::<Vec<_>>()
    };
    let required_rule = KNOWN
        .iter()
        .map(|(name, _)| (*name, *name != "rule"))
        .collect::<Vec<_>>();
    for (case, fields) in [
        // A fifth matcher, or any other new field, must be classified first.
        ("added field", with(("css_units", true))),
        ("missing matcher", without("markdown_units")),
        ("missing shared field", without("within")),
        ("required matcher", required_rule),
    ] {
        assert!(exclusive_matcher(wire(&fields)).is_err(), "{case}");
    }
    assert!(exclusive_matcher(None).is_err(), "unregistered wire");
}
