use crate::*;
use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, SeqAccess, Visitor},
};
use std::{collections::BTreeSet, fmt};

struct UniqueJson;
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
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
            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                while sequence.next_element::<UniqueJson>()?.is_some() {}
                Ok(UniqueJson)
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut names = BTreeSet::new();
                while let Some(name) = map.next_key::<String>()? {
                    if !names.insert(name) {
                        return Err(serde::de::Error::custom("duplicate key"));
                    }
                    map.next_value::<UniqueJson>()?;
                }
                Ok(UniqueJson)
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}
pub fn decode_config(text: &str, location: &ConfigLocation) -> Result<RulesConfig, LintInvalid> {
    let failure = |error: &serde_json::Error, path: Vec<String>| {
        let message = error.to_string();
        let code = if message.starts_with("duplicate key") {
            if path.first().is_some_and(|key| key == "rule_sets") {
                LintInvalidCode::DuplicateEntry
            } else {
                LintInvalidCode::DuplicateKey
            }
        } else if message.starts_with("unknown field") {
            LintInvalidCode::UnknownField
        } else if message.starts_with("old severity word") {
            LintInvalidCode::OldSeverityWord
        } else if message.contains("invalid glob") {
            LintInvalidCode::BadGlob
        } else if message.contains("rule selector") || message.contains("entry names") {
            LintInvalidCode::UnknownSelector
        } else {
            LintInvalidCode::Syntax
        };
        LintInvalid {
            file: location.file.clone(),
            line: Some(u32::try_from(error.line()).unwrap_or(u32::MAX)),
            column: Some(u32::try_from(error.column()).unwrap_or(u32::MAX)),
            json_path: path,
            code,
            message: format!(
                "Invalid sunstack.lint.json: {}",
                migration_message(&message)
            ),
        }
    };
    let mut uniqueness = serde_json::Deserializer::from_str(text);
    let _: UniqueJson = serde_path_to_error::deserialize(&mut uniqueness)
        .map_err(|error| failure(error.inner(), path_fields(error.path())))?;
    uniqueness.end().map_err(|error| failure(&error, vec![]))?;
    let mut decoder = serde_json::Deserializer::from_str(text);
    let mut config: RulesConfig = serde_path_to_error::deserialize(&mut decoder)
        .map_err(|error| failure(error.inner(), path_fields(error.path())))?;
    decoder.end().map_err(|error| failure(&error, vec![]))?;
    config.source = Some(crate::config_contracts::ConfigSource {
        location: location.clone(),
        text: text.into(),
    });
    validate_config_shape(&config)?;
    Ok(config)
}
pub(crate) fn validate_config_shape(config: &RulesConfig) -> Result<(), LintInvalid> {
    if config.rulebook.is_empty()
        || config.check.files.is_empty()
        || config.evidence.kinds.is_empty()
    {
        return Err(config_error(
            config,
            "rulebook",
            LintInvalidCode::Syntax,
            "rulebook, check.files and evidence.kinds must be nonempty",
        ));
    }
    if config
        .rulebook
        .iter()
        .any(|pair| pair.markdown.as_os_str().is_empty() || pair.rules.as_os_str().is_empty())
        || config.evidence.ast_grep.as_os_str().is_empty()
    {
        return Err(config_error(
            config,
            "rulebook",
            LintInvalidCode::Syntax,
            "configured paths must be nonempty",
        ));
    }
    if config
        .limits
        .as_ref()
        .and_then(|limits| limits.evidence_max_chars)
        .is_some_and(|value| value.get() < 1000)
    {
        return Err(config_error(
            config,
            "evidence_max_chars",
            LintInvalidCode::Syntax,
            "evidence_max_chars must be at least 1000",
        ));
    }

    Ok(())
}
fn path_fields(path: &serde_path_to_error::Path) -> Vec<String> {
    path.iter()
        .filter_map(|segment| match segment {
            serde_path_to_error::Segment::Map { key } => Some(key.clone()),
            serde_path_to_error::Segment::Seq { index } => Some(index.to_string()),
            _ => None,
        })
        .collect()
}
pub(crate) fn config_error(
    config: &RulesConfig,
    key: &str,
    code: LintInvalidCode,
    message: impl Into<String>,
) -> LintInvalid {
    let (file, line, column) = match &config.source {
        Some(source) => {
            let quoted = serde_json::to_string(key).unwrap_or_default();
            let position = source.text.find(&quoted).unwrap_or(0);
            let prefix = &source.text[..position];
            (
                source.location.file.clone(),
                u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count() + 1)
                    .unwrap_or(u32::MAX),
                u32::try_from(
                    prefix
                        .rsplit('\n')
                        .next()
                        .map_or(1, |tail| tail.chars().count() + 1),
                )
                .unwrap_or(u32::MAX),
            )
        }
        None => (std::path::PathBuf::from("sunstack.lint.json"), 1, 1),
    };
    LintInvalid {
        file,
        line: Some(line),
        column: Some(column),
        json_path: vec![key.into()],
        code,
        message: message.into(),
    }
}

fn migration_message(message: &str) -> String {
    for (old, replacement) in [
        ("rule_sets", "rulebook"),
        ("rules", "severity in the rule file"),
        ("overrides", "severity in the rule file"),
        ("ignore_patterns", "check.ignore"),
        ("backend", "decision_model"),
    ] {
        if message.starts_with(&format!("unknown field `{old}`")) {
            return format!("{old} was replaced by {replacement}");
        }
    }
    message.into()
}
