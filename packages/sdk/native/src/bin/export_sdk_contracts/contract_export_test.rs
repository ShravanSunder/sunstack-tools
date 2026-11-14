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

#[test]
fn document_roots_keep_the_concrete_report_status_bindings() {
    let exported = export().unwrap();
    let roots = &exported.wide_integers["document_roots"];
    let roots = roots.as_object().unwrap();
    assert_eq!(roots.len(), 8);
    assert_eq!(roots["report"]["ref"], "SdkReport");
    assert_eq!(roots["report"]["arguments"]["TStatus"]["ref"], "LintStatus");
    assert_eq!(roots["inspection_report"]["ref"], "InspectionReport");
    assert_eq!(
        roots["inspection_report"]["arguments"]["TInspectionStatus"]["ref"],
        "InspectionStatus"
    );
    assert_eq!(roots["configuration"]["ref"], "Configuration");
}

#[test]
fn private_same_path_u64_and_u32_roots_have_distinct_widening_shapes() {
    let root = |primitive| {
        Struct::named()
            .field(
                "shared",
                Field::new(
                    Struct::named()
                        .field("count", Field::new(DataType::Primitive(primitive)))
                        .build(),
                ),
            )
            .build()
    };
    let types = Types::default();
    assert_eq!(
        wide_shape(&root(Primitive::u64), &types),
        Some(json!({"object":{"shared":{"object":{"count":"bigint"}}}}))
    );
    assert_eq!(wide_shape(&root(Primitive::u32), &types), None);
}

#[test]
fn inside_scope_exclusion_follows_the_registered_rust_variant_and_refuses_shape_drift() {
    let types = Types::default().register::<sunstack_evidence::FieldSource>();
    let original = types
        .into_sorted_iter()
        .find(|named| named.name == "FieldSource")
        .and_then(|named| named.ty.clone())
        .unwrap();
    let mut current = original.clone();
    exclude_inside_scope(Some(&mut current)).unwrap();
    let DataType::Enum(current) = current else {
        panic!("FieldSource enum required");
    };
    let (_, inside) = current
        .variants
        .iter()
        .find(|(name, _)| name == "Inside")
        .unwrap();
    let Fields::Named(fields) = &inside.fields else {
        panic!("Inside named fields required");
    };
    assert_eq!(fields.fields.len(), 2);
    assert!(
        fields
            .fields
            .iter()
            .any(|(name, field)| name == "scope" && field.optional)
    );

    // An additional real field needs a new policy; generation must not drop it.
    let mut drifted = original;
    let DataType::Enum(enumeration) = &mut drifted else {
        panic!("FieldSource enum required");
    };
    let (_, inside) = enumeration
        .variants
        .iter_mut()
        .find(|(name, _)| name == "Inside")
        .unwrap();
    let Fields::Named(fields) = &mut inside.fields else {
        panic!("Inside named fields required");
    };
    fields.fields.push(("new_field".into(), field(false)));
    assert!(exclude_inside_scope(Some(&mut drifted)).is_err());
    assert!(exclude_inside_scope(None).is_err());
}
