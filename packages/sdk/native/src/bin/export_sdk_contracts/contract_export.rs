//! Export policy for SDK declarations. Rust types and constructors stay unchanged:
//! full-width integers become `bigint` in TypeScript and are recorded so the SDK
//! decodes replies without rounding.
use serde_json::{Map, Value, json};
use specta::{
    Format, Types,
    datatype::{DataType, Fields, NamedReferenceType, Primitive, Reference},
};
use std::{borrow::Cow, error::Error};
use sunstack::sdk_host::{
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

impl Format for SdkFormat {
    fn map_types(&self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        let mut mapped = specta_serde::Format.map_types(types)?.into_owned();
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

pub fn export() -> Result<Exported, Box<dyn Error>> {
    let types = Types::default()
        .register::<Configuration>()
        .register::<ResolvedConfiguration>()
        .register::<LoadOptions>()
        .register::<RunRequest>()
        .register::<ContractInfo>()
        .register::<LoadOutcome>()
        .register::<CheckOutcome>()
        .register::<RunOutcome>()
        .register::<sunstack_evidence::EvidenceLanguage>();
    let declarations = specta_typescript::Typescript::default().export(&types, SdkFormat)?;
    let serde_types = specta_serde::Format
        .map_types(&types)
        .map_err(|error| error.to_string())?
        .into_owned();
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
        wide_integers: json!({"definitions": definitions}),
    })
}
