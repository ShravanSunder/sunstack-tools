use crate::EvalError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

macro_rules! eval_identity {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(#[schemars(length(min = 1))] String);
        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, EvalError> {
                let value = value.into();
                if value.is_empty() {
                    return Err(EvalError::Invalid {
                        field: stringify!($name).into(),
                    });
                }
                Ok(Self(value))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
                Self::new(String::deserialize(decoder)?).map_err(serde::de::Error::custom)
            }
        }
    };
}
eval_identity!(CaseId);
eval_identity!(CaseSourceId);
eval_identity!(ProjectKey);
eval_identity!(ContentIdentity);
eval_identity!(CaseSetIdentity);
