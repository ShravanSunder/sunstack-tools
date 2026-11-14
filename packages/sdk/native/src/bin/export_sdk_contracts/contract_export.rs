//! Export policy for SDK declarations. Rust types and constructors stay unchanged:
//! full-width integers become `bigint` in TypeScript and are recorded so the SDK
//! decodes replies without rounding.
use serde_json::{Map, Value, json};
use specta::{
    Format, Types,
    datatype::{
        DataType, Enum, Field, Fields, NamedDataType, NamedReferenceType, Primitive, Reference,
        Variant,
    },
};
use std::{borrow::Cow, error::Error};
use sunstack::sdk_host::{
    document_codec::{
        DeserializationOutcome, DeserializeSdkRequest, SdkDocument, SdkDocumentValue,
    },
    inspection_projection::InspectRequest,
    report_projection::RunOutcome,
    sdk_contracts::{
        CheckOutcome, Configuration, ContractInfo, LoadOptions, LoadOutcome, ResolvedConfiguration,
        RunRequest,
    },
};

pub struct Exported {
    pub declarations: String,
    pub wide_integers: Value,
}

#[path = "exclusive_fields_export.rs"]
mod exclusive_fields_export;

fn is_wide(primitive: &Primitive) -> bool {
    matches!(
        primitive,
        Primitive::u64
            | Primitive::i64
            | Primitive::usize
            | Primitive::isize
            | Primitive::u128
            | Primitive::i128
    )
}

/// Serde's wire shapes, with full-width integers as `bigint`.
struct SdkFormat;

impl SdkFormat {
    fn rewrite(datatype: &mut DataType) {
        match datatype {
            DataType::Primitive(primitive) if is_wide(primitive) => {
                *datatype = DataType::Reference(specta_typescript::define("bigint"));
            }
            // Inputs refuse non-finite numbers before dispatch; floats are plain numbers.
            DataType::Primitive(Primitive::f32 | Primitive::f64) => {
                *datatype = DataType::Reference(specta_typescript::define("number"));
            }
            DataType::List(list) => Self::rewrite(&mut list.ty),
            DataType::Map(map) => {
                // JSON object keys are strings, including integer-keyed maps.
                if matches!(map.key_ty(), DataType::Primitive(primitive) if is_wide(primitive)) {
                    *map.key_ty_mut() = DataType::Primitive(Primitive::str);
                } else {
                    Self::rewrite(map.key_ty_mut());
                }
                Self::rewrite(map.value_ty_mut());
            }
            DataType::Struct(structure) => Self::rewrite_fields(&mut structure.fields),
            DataType::Enum(enumeration) => {
                // A JSON number is a finite float or an exact integer. Both cross the wire, and
                // replies widen integers to bigint, so the declaration keeps both branches
                // instead of collapsing to number.
                enumeration
                    .attributes
                    .remove(specta::markers::FINITE_NUMBER_MARKER);
                for (_, variant) in &mut enumeration.variants {
                    Self::rewrite_fields(&mut variant.fields);
                }
            }
            DataType::Tuple(tuple) => tuple.elements.iter_mut().for_each(Self::rewrite),
            DataType::Nullable(inner) => Self::rewrite(inner),
            DataType::Intersection(members) => members.iter_mut().for_each(Self::rewrite),
            DataType::Reference(Reference::Named(reference)) => match &mut reference.inner {
                NamedReferenceType::Reference { generics, .. } => {
                    for (_, argument) in generics {
                        Self::rewrite(argument);
                    }
                }
                NamedReferenceType::Inline { dt, .. } => Self::rewrite(dt),
                NamedReferenceType::Recursive(_) => {}
            },
            DataType::Primitive(_)
            | DataType::Generic(_)
            | DataType::Reference(Reference::Opaque(_)) => {}
        }
    }

    fn rewrite_fields(fields: &mut Fields) {
        match fields {
            Fields::Unit => {}
            Fields::Unnamed(unnamed) => unnamed
                .fields
                .iter_mut()
                .filter_map(|field| field.ty.as_mut())
                .for_each(Self::rewrite),
            Fields::Named(named) => named
                .fields
                .iter_mut()
                .filter_map(|(_, field)| field.ty.as_mut())
                .for_each(Self::rewrite),
        }
    }
}

/// Engine types whose Rust names repeat in another module, with their exported name.
///
/// The HTML unit selection keeps a private `SelectionWire` beside the kind `SelectionWire`,
/// and the HTML and Markdown selections each keep a private `UnitOnly`;
/// TypeScript needs one name per declaration, and the Rust name stays unchanged.
const DISTINCT_NAMES: &[(&str, &str, &str)] = &[
    (
        "html_selection_contracts",
        "SelectionWire",
        "HtmlUnitSelectionWire",
    ),
    (
        "markdown_selection_contracts",
        "UnitOnly",
        "MarkdownUnitOnly",
    ),
];

fn distinct_names(types: &mut Types) {
    types.iter_mut(|definition| {
        for (module, name, exported) in DISTINCT_NAMES {
            if definition.name == *name && definition.module_path.ends_with(module) {
                definition.name = Cow::Borrowed(exported);
            }
        }
    });
}

impl Format for SdkFormat {
    fn map_types(&self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        let mut mapped = specta_serde::Format.map_types(types)?.into_owned();
        distinct_names(&mut mapped);
        mapped.iter_mut(|definition| {
            if let Some(datatype) = &mut definition.ty {
                Self::rewrite(datatype);
            }
        });
        Ok(Cow::Owned(mapped))
    }

    fn map_type(
        &self,
        types: &Types,
        datatype: &DataType,
    ) -> Result<Cow<'_, DataType>, specta::FormatError> {
        let mut mapped = specta_serde::Format.map_type(types, datatype)?.into_owned();
        Self::rewrite(&mut mapped);
        Ok(Cow::Owned(mapped))
    }
}

/// Where full-width integers sit in one serde wire shape; `None` when nowhere.
fn wide_shape(datatype: &DataType, types: &Types) -> Option<Value> {
    match datatype {
        DataType::Primitive(primitive) => is_wide(primitive).then(|| json!("bigint")),
        DataType::List(list) => wide_shape(&list.ty, types).map(|item| json!({"list": item})),
        DataType::Map(map) => wide_shape(map.value_ty(), types).map(|value| json!({"map": value})),
        DataType::Struct(structure) => fields_shape(&structure.fields, types),
        DataType::Enum(enumeration) => {
            let variants = enumeration
                .variants
                .iter()
                .filter(|(_, variant)| !variant.skip)
                .filter_map(|(_, variant)| fields_shape(&variant.fields, types))
                .collect::<Vec<_>>();
            (!variants.is_empty()).then(|| json!({"union": variants}))
        }
        DataType::Tuple(tuple) => {
            let elements = tuple
                .elements
                .iter()
                .map(|element| wide_shape(element, types).unwrap_or(Value::Null))
                .collect::<Vec<_>>();
            elements
                .iter()
                .any(|element| !element.is_null())
                .then(|| json!({"tuple": elements}))
        }
        DataType::Nullable(inner) => wide_shape(inner, types),
        DataType::Intersection(members) => {
            let members = members
                .iter()
                .filter_map(|member| wide_shape(member, types))
                .collect::<Vec<_>>();
            (!members.is_empty()).then(|| json!({"all": members}))
        }
        DataType::Generic(generic) => Some(json!({"generic": generic.name().to_string()})),
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Reference { generics, .. } => {
                let named = types.get(reference)?;
                let arguments = generics
                    .iter()
                    .map(|(generic, argument)| {
                        (
                            generic.name().to_string(),
                            wide_shape(argument, types).unwrap_or(Value::Null),
                        )
                    })
                    .collect::<Map<_, _>>();
                Some(json!({"ref": named.name, "arguments": arguments}))
            }
            NamedReferenceType::Inline { dt, .. } => wide_shape(dt, types),
            NamedReferenceType::Recursive(_) => None,
        },
        DataType::Reference(Reference::Opaque(_)) => None,
    }
}

fn fields_shape(fields: &Fields, types: &Types) -> Option<Value> {
    match fields {
        Fields::Unit => None,
        Fields::Unnamed(unnamed) => match unnamed.fields.as_slice() {
            [single] => single.ty.as_ref().and_then(|ty| wide_shape(ty, types)),
            many => {
                let elements = many
                    .iter()
                    .map(|field| {
                        field
                            .ty
                            .as_ref()
                            .and_then(|ty| wide_shape(ty, types))
                            .unwrap_or(Value::Null)
                    })
                    .collect::<Vec<_>>();
                elements
                    .iter()
                    .any(|element| !element.is_null())
                    .then(|| json!({"tuple": elements}))
            }
        },
        Fields::Named(named) => {
            let object = named
                .fields
                .iter()
                .filter_map(|(name, field)| {
                    field
                        .ty
                        .as_ref()
                        .and_then(|ty| wide_shape(ty, types))
                        .map(|shape| (name.to_string(), shape))
                })
                .collect::<Map<_, _>>();
            (!object.is_empty()).then(|| json!({"object": object}))
        }
    }
}

/// Derive each document root from the serde-transformed payload, preserving generic arguments.
fn document_root_shapes(types: &Types) -> Result<Map<String, Value>, Box<dyn Error>> {
    let document = types
        .into_sorted_iter()
        .find(|named| named.name == "SdkDocumentValue")
        .and_then(|named| named.ty.as_ref())
        .ok_or("SdkDocumentValue is not registered")?;
    let DataType::Enum(document) = document else {
        return Err("SdkDocumentValue is not an enum".into());
    };
    let expected = [
        "configuration",
        "loaded_configuration",
        "load_outcome",
        "check_outcome",
        "run_outcome",
        "inspection_outcome",
        "report",
        "inspection_report",
    ];
    let mut roots = Map::new();
    for (kind, variant) in &document.variants {
        if !expected.contains(&kind.as_ref()) || roots.contains_key(kind.as_ref()) {
            return Err(format!("unknown or duplicate SdkDocumentValue root {kind}").into());
        }
        let Fields::Named(fields) = &variant.fields else {
            return Err(format!("SdkDocumentValue root {kind} is not adjacent-tagged").into());
        };
        if fields.fields.len() != 2 || !fields.fields.iter().any(|(name, _)| name == "kind") {
            return Err(format!("SdkDocumentValue root {kind} has unexpected fields").into());
        }
        let payload = fields
            .fields
            .iter()
            .find(|(name, _)| name == "value")
            .and_then(|(_, field)| field.ty.as_ref())
            .ok_or_else(|| format!("SdkDocumentValue root {kind} has no payload"))?;
        let shape = wide_shape(payload, types)
            .ok_or_else(|| format!("SdkDocumentValue root {kind} has no known wide shape"))?;
        roots.insert(kind.to_string(), shape);
    }
    if roots.len() != expected.len() {
        return Err("SdkDocumentValue roots are incomplete".into());
    }
    Ok(roots)
}

/// Give the evidence selection question its admitted shape.
///
/// The evidence crate keeps the question as model-free JSON so it does not depend on the
/// decision crate; rule admission then decodes it as a `JudgmentQuestion` and accepts only
/// the `noul` variant. The SDK declares that admitted shape: the Rust `JudgmentQuestion`
/// definition restricted to its `Noul` variant, exported as `SelectionQuestion`.
fn select_question_shape(types: &mut Types) -> Result<(), Box<dyn Error>> {
    let question = types
        .into_sorted_iter()
        .find(|named| named.name == "JudgmentQuestion")
        .and_then(|named| named.ty.clone())
        .ok_or("JudgmentQuestion is not registered")?;
    let DataType::Enum(mut noul) = question else {
        return Err("JudgmentQuestion is not an enum".into());
    };
    noul.variants.retain(|(name, _)| name == "Noul");
    if noul.variants.len() != 1 {
        return Err("JudgmentQuestion has no Noul variant".into());
    }
    let selection = NamedDataType::new("SelectionQuestion", types, |_, named| {
        named.docs = Cow::Borrowed(" The judgment question an evidence selection asks: noul only.");
        named.ty = Some(DataType::Enum(noul));
    });
    let mut replaced = false;
    types.iter_mut(|named| {
        if named.name != "SelectionWire" || !named.module_path.ends_with("evidence_kind_contracts")
        {
            return;
        }
        if let Some(DataType::Struct(structure)) = &mut named.ty
            && let Fields::Named(fields) = &mut structure.fields
        {
            for (name, field) in &mut fields.fields {
                if name == "question" {
                    field.ty = Some(DataType::Reference(selection.reference(vec![])));
                    replaced = true;
                }
            }
        }
    });
    if replaced {
        Ok(())
    } else {
        Err("SelectionWire.question was not found".into())
    }
}

/// The fields an evidence kind shares whatever its matcher, as `EvidenceKindWire` names them.
const SHARED_KEYS: [&str; 4] = ["fields", "max_chars", "select", "within"];

/// The exclusive matcher keys of an evidence kind, as `EvidenceKindWire` names them.
const MATCHER_KEYS: [(&str, &str); 4] = [
    ("rule", "Rule"),
    ("inline_rule", "InlineRule"),
    ("html_units", "HtmlUnits"),
    ("markdown_units", "MarkdownUnits"),
];

/// Declare an evidence kind's matcher as exactly one of its four keys.
///
/// `EvidenceKindWire` keeps the four matcher keys as separate optional fields so serde can
/// decode each with its own decoder; its JSON Schema and `TryFrom` then admit exactly one.
/// The SDK declares that admitted shape: the kind's other fields, intersected with an
/// untagged union whose variants each require one matcher key with its Rust field type.
/// TypeScript marks the other keys of each variant `never`, so a kind with no matcher key
/// or with two is refused. Every `EvidenceKindWire` field must be a known matcher or shared
/// key: a field added to the Rust wire fails generation until it is classified here.
fn exclusive_matcher_shape(types: &mut Types) -> Result<(), Box<dyn Error>> {
    let mut outcome: Result<bool, String> = Ok(false);
    types.iter_mut(|named| {
        if named.name != "EvidenceKindWire" || !named.module_path.ends_with("kind_matcher_decoding")
        {
            return;
        }
        outcome = exclusive_matcher(named.ty.take()).map(|declared| {
            named.ty = Some(declared);
            true
        });
    });
    match outcome? {
        true => Ok(()),
        false => Err("EvidenceKindWire is not registered".into()),
    }
}

fn exclusive_matcher(wire: Option<DataType>) -> Result<DataType, String> {
    let Some(DataType::Struct(mut shared)) = wire else {
        return Err("EvidenceKindWire is not a struct".into());
    };
    let Fields::Named(fields) = &mut shared.fields else {
        return Err("EvidenceKindWire has no named fields".into());
    };
    let mut matchers = Vec::new();
    fields.fields.retain(|(name, field)| {
        let is_matcher = MATCHER_KEYS.iter().any(|(key, _)| name == key);
        if is_matcher {
            matchers.push((name.to_string(), field.clone()));
        }
        !is_matcher
    });
    let mut choice = Enum::default();
    for (key, variant) in MATCHER_KEYS {
        let Some((_, field)) = matchers.iter().find(|(name, _)| name == key) else {
            return Err(format!("EvidenceKindWire has no `{key}` matcher field"));
        };
        if !field.optional {
            return Err(format!(
                "EvidenceKindWire `{key}` is not optional; its exclusivity is no longer a choice"
            ));
        }
        let ty = field
            .ty
            .clone()
            .ok_or_else(|| format!("EvidenceKindWire `{key}` is skipped"))?;
        choice.variants.push((
            Cow::Borrowed(variant),
            Variant::named().field(key, Field::new(ty)).build(),
        ));
    }
    if matchers.len() != MATCHER_KEYS.len() {
        return Err("EvidenceKindWire matcher fields do not match the four matcher keys".into());
    }
    let shared_names = fields
        .fields
        .iter()
        .map(|(name, _)| name.to_string())
        .collect::<Vec<_>>();
    let mut sorted_shared = shared_names.clone();
    sorted_shared.sort();
    let mut known_shared = SHARED_KEYS.map(str::to_string).to_vec();
    known_shared.sort();
    if sorted_shared != known_shared {
        return Err(format!(
            "EvidenceKindWire fields {shared_names:?} are not exactly the known shared keys \
             {SHARED_KEYS:?}; classify any new field as a matcher or a shared field"
        ));
    }
    choice
        .attributes
        .insert(specta::markers::SERDE_CONTAINER_UNTAGGED, true);
    Ok(DataType::Intersection(vec![
        DataType::Struct(shared),
        DataType::Enum(choice),
    ]))
}

/// The unscoped and scoped `inside` branches share a wire tag. Structural TypeScript
/// assignments must not accept a scoped value through the unscoped branch.
fn exclusive_inside_scope_shape(types: &mut Types) -> Result<(), Box<dyn Error>> {
    let mut outcome = Err("FieldSource is not registered".to_string());
    types.iter_mut(|named| {
        if named.name == "FieldSource" && named.module_path.ends_with("evidence_kind_contracts") {
            outcome = exclude_inside_scope(named.ty.as_mut());
        }
    });
    outcome.map_err(Into::into)
}

fn exclude_inside_scope(datatype: Option<&mut DataType>) -> Result<(), String> {
    let Some(DataType::Enum(enumeration)) = datatype else {
        return Err("FieldSource is not an enum".into());
    };
    let scoped = enumeration
        .variants
        .iter()
        .find(|(name, _)| name == "ScopedInside")
        .ok_or("FieldSource has no ScopedInside variant")?;
    let Fields::Named(scoped_fields) = &scoped.1.fields else {
        return Err("ScopedInside has no named fields".into());
    };
    let (scope_name, scope_field) = scoped_fields
        .fields
        .iter()
        .find(|(name, _)| name == "scope")
        .ok_or("ScopedInside has no scope field")?;
    if scope_field.optional || scope_field.ty.is_none() {
        return Err("ScopedInside scope is no longer required".into());
    }
    let scope_name = scope_name.clone();
    let mut excluded_scope = Field::new(DataType::Reference(specta_typescript::define("never")));
    excluded_scope.optional = true;
    let (_, unscoped) = enumeration
        .variants
        .iter_mut()
        .find(|(name, _)| name == "Inside")
        .ok_or("FieldSource has no Inside variant")?;
    let Fields::Named(fields) = &mut unscoped.fields else {
        return Err("Inside has no named fields".into());
    };
    if fields.fields.len() != 1 || fields.fields[0].0 != "of" {
        return Err("Inside no longer has exactly its flattened source field".into());
    }
    fields.fields.push((scope_name, excluded_scope));
    Ok(())
}

pub fn export() -> Result<Exported, Box<dyn Error>> {
    let mut types = Types::default()
        .register::<Configuration>()
        .register::<ResolvedConfiguration>()
        .register::<LoadOptions>()
        .register::<RunRequest>()
        .register::<ContractInfo>()
        .register::<LoadOutcome>()
        .register::<CheckOutcome>()
        .register::<RunOutcome>()
        .register::<SdkDocumentValue>()
        .register::<SdkDocument<SdkDocumentValue>>()
        .register::<DeserializeSdkRequest>()
        .register::<InspectRequest>()
        .register::<DeserializationOutcome>()
        .register::<sunstack_evidence::EvidenceLanguage>();
    select_question_shape(&mut types)?;
    exclusive_matcher_shape(&mut types)?;
    exclusive_inside_scope_shape(&mut types)?;
    exclusive_fields_export::exclude_other_fields(
        &mut types,
        "CountBound",
        "decision_graph_contracts",
    )?;
    exclusive_fields_export::exclude_other_fields(&mut types, "SectionScope", "section_scope")?;
    let declarations = specta_typescript::Typescript::default().export(&types, SdkFormat)?;
    let mut serde_types = specta_serde::Format
        .map_types(&types)
        .map_err(|error| error.to_string())?
        .into_owned();
    distinct_names(&mut serde_types);
    let document_roots = document_root_shapes(&serde_types)?;
    let mut definitions = Map::new();
    for named in serde_types.into_sorted_iter() {
        let generics = named
            .generics
            .iter()
            .map(|generic| Value::String(generic.name.to_string()))
            .collect::<Vec<_>>();
        if let Some(shape) = named
            .ty
            .as_ref()
            .and_then(|ty| wide_shape(ty, &serde_types))
        {
            definitions.insert(
                named.name.to_string(),
                json!({"generics": generics, "shape": shape}),
            );
        }
    }
    Ok(Exported {
        declarations: format!(
            "// Generated by export-sdk-contracts from Rust-owned contracts. Do not edit.\n\
             import type {{ AstMatcher, AstUtility }} from './ast-grep.ts';\n\n{declarations}"
        ),
        wide_integers: json!({"definitions": definitions, "document_roots": document_roots}),
    })
}

#[cfg(test)]
#[path = "contract_export_test.rs"]
mod tests;
