//! Two passes mirror rules' private decoder: reject duplicate keys, then decode the owned shape.
use crate::EvalError;
use serde::{
    Deserialize, Deserializer,
    de::{DeserializeOwned, MapAccess, SeqAccess, Visitor},
};
use std::{collections::BTreeSet, fmt};
struct UniqueJson;
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueJson;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("JSON with unique object keys")
            }
            fn visit_bool<E>(self, _: bool) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_i64<E>(self, _: i64) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_u64<E>(self, _: u64) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_f64<E>(self, _: f64) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_str<E>(self, _: &str) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                while sequence.next_element::<UniqueJson>()?.is_some() {}
                Ok(UniqueJson)
            }
            fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Self::Value, A::Error> {
                let mut keys = BTreeSet::new();
                while let Some(key) = object.next_key::<String>()? {
                    if !keys.insert(key) {
                        return Err(serde::de::Error::custom("duplicate field"));
                    }
                    object.next_value::<UniqueJson>()?;
                }
                Ok(UniqueJson)
            }
        }
        decoder.deserialize_any(UniqueVisitor)
    }
}
pub(crate) fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, EvalError> {
    decode_with_forbidden(bytes, &[])
}
pub(crate) fn decode_with_forbidden<T: DeserializeOwned>(
    bytes: &[u8],
    forbidden: &[&str],
) -> Result<T, EvalError> {
    let mut unique = serde_json::Deserializer::from_slice(bytes);
    let _: UniqueJson =
        serde_path_to_error::deserialize(&mut unique).map_err(|error| invalid(error.path()))?;
    unique.end().map_err(|_| EvalError::Invalid {
        field: "json".into(),
    })?;
    if !forbidden.is_empty() {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| EvalError::Invalid {
                field: "json".into(),
            })?;
        for field in forbidden {
            if value.get(*field).is_some() {
                return Err(EvalError::ForbiddenField {
                    field: (*field).into(),
                });
            }
        }
    }
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let value =
        serde_path_to_error::deserialize(&mut decoder).map_err(|error| invalid(error.path()))?;
    decoder.end().map_err(|_| EvalError::Invalid {
        field: "json".into(),
    })?;
    Ok(value)
}
fn invalid(path: &serde_path_to_error::Path) -> EvalError {
    // Free author keys and values never become diagnostic text.
    let field = path
        .iter()
        .filter_map(|part| match part {
            serde_path_to_error::Segment::Seq { index } => Some(index.to_string()),
            serde_path_to_error::Segment::Map { key }
                if matches!(
                    key.as_str(),
                    "case_set"
                        | "backend"
                        | "limits"
                        | "execution"
                        | "attempts"
                        | "request_budget"
                        | "recipes"
                        | "repositories"
                        | "discovery"
                        | "entry"
                        | "payload_schemas"
                        | "name"
                        | "rule"
                        | "split"
                        | "sources"
                        | "labels"
                        | "source_files"
                        | "definition_files"
                        | "path"
                        | "content_identity"
                        | "source_id"
                        | "source"
                        | "evidence"
                        | "case_id"
                        | "session"
                        | "provider"
                        | "project"
                        | "source_anchor"
                        | "question"
                        | "label"
                ) =>
            {
                Some(key.clone())
            }
            _ => None,
        })
        .take(32)
        .collect::<Vec<_>>()
        .join(".");
    EvalError::Invalid {
        field: if field.is_empty() {
            "json".into()
        } else {
            field
        },
    }
}
