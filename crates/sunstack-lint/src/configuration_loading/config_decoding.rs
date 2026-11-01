use crate::*;
use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, SeqAccess, Visitor},
};
use std::{collections::BTreeSet, fmt};
use sunstack_evidence::{ConfigPath, ConfigPathSegment};

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
    let failure = |error: &serde_json::Error, path: ConfigPath| {
        let message = error.to_string();
        let code = if message.starts_with("duplicate key") {
            if matches!(path.segments().first(), Some(ConfigPathSegment::Key(key)) if key == "rule_sets")
            {
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
        let field_context = match path.segments() {
            [
                ConfigPathSegment::Key(evidence),
                ConfigPathSegment::Key(kinds),
                _,
                ConfigPathSegment::Key(fields),
                _,
                ..,
            ] if evidence == "evidence" && kinds == "kinds" && fields == "fields" => {
                format!(" ({path})")
            }
            _ => String::new(),
        };
        LintInvalid {
            file: location.file.clone(),
            line: Some(u32::try_from(error.line()).unwrap_or(u32::MAX)),
            column: Some(u32::try_from(error.column()).unwrap_or(u32::MAX)),
            json_path: path
                .segments()
                .iter()
                .map(|segment| match segment {
                    ConfigPathSegment::Key(key) => key.clone(),
                    ConfigPathSegment::Index(index) => index.to_string(),
                })
                .collect(),
            code,
            message: format!(
                "Invalid sunstack.lint.json: {}{field_context}",
                migration_message(&message)
            ),
        }
    };
    let mut uniqueness = serde_json::Deserializer::from_str(text);
    let _: UniqueJson = serde_path_to_error::deserialize(&mut uniqueness)
        .map_err(|error| failure(error.inner(), path_fields(error.path())))?;
    uniqueness
        .end()
        .map_err(|error| failure(&error, ConfigPath::default()))?;
    let mut decoder = serde_json::Deserializer::from_str(text);
    let mut config: RulesConfig = serde_path_to_error::deserialize(&mut decoder)
        .map_err(|error| failure(error.inner(), path_fields(error.path())))?;
    decoder
        .end()
        .map_err(|error| failure(&error, ConfigPath::default()))?;
    config.source = Some(
        crate::configuration_loading::config_contracts::ConfigSource {
            location: location.clone(),
            text: text.into(),
        },
    );
    validate_config_shape(&config)?;
    Ok(config)
}
pub(crate) fn validate_config_shape(config: &RulesConfig) -> Result<(), LintInvalid> {
    let empty_collection = if config.rulebook.is_empty() {
        Some(ConfigPath::keys(["rulebook"]))
    } else if config.check.files.is_empty() {
        Some(ConfigPath::keys(["check", "files"]))
    } else if config.evidence.kinds.is_empty() {
        Some(ConfigPath::keys(["evidence", "kinds"]))
    } else {
        None
    };
    if let Some(path) = empty_collection {
        return Err(config_error(
            config,
            &path,
            LintInvalidCode::Syntax,
            "rulebook, check.files and evidence.kinds must be nonempty",
        ));
    }
    let empty_pair_path = config
        .rulebook
        .iter()
        .enumerate()
        .find_map(|(index, pair)| {
            let field = if pair.markdown.as_os_str().is_empty() {
                Some("markdown")
            } else if pair.rules.as_os_str().is_empty() {
                Some("rules")
            } else {
                None
            };
            field.map(|field| {
                ConfigPath::keys(["rulebook"])
                    .with_index(index)
                    .with_key(field)
            })
        });
    let empty_path = empty_pair_path.or_else(|| {
        config
            .evidence
            .ast_grep
            .as_os_str()
            .is_empty()
            .then(|| ConfigPath::keys(["evidence", "ast_grep"]))
    });
    if let Some(path) = empty_path {
        return Err(config_error(
            config,
            &path,
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
            &ConfigPath::keys(["limits", "evidence_max_chars"]),
            LintInvalidCode::Syntax,
            "evidence_max_chars must be at least 1000",
        ));
    }

    Ok(())
}
fn path_fields(path: &serde_path_to_error::Path) -> ConfigPath {
    ConfigPath::new(path.iter().filter_map(|segment| match segment {
        serde_path_to_error::Segment::Map { key } => Some(ConfigPathSegment::Key(key.clone())),
        serde_path_to_error::Segment::Seq { index } => Some(ConfigPathSegment::Index(*index)),
        _ => None,
    }))
}
pub(crate) fn config_error(
    config: &RulesConfig,
    path: &ConfigPath,
    code: LintInvalidCode,
    message: impl Into<String>,
) -> LintInvalid {
    let message = message.into();
    let (file, line, column) = match &config.source {
        Some(source) => {
            let position = config_field_position(&source.text, path).unwrap_or(0);
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
        json_path: vec![path.to_string()],
        code,
        message,
    }
}

// Locate literal object keys and indexed array values without parsing their
// displayed diagnostic spelling back into configuration structure.
fn config_field_position(text: &str, path: &ConfigPath) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut search_start = 0;
    let mut position = 0;
    for segment in path.segments() {
        let mut cursor = search_start;
        while bytes
            .get(cursor)
            .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b':')
        {
            cursor += 1;
        }
        let container_is_array = match bytes.get(cursor)? {
            b'{' => false,
            b'[' => true,
            _ => return None,
        };
        let mut depth = 0_usize;
        let mut found = None;
        let mut element_index = 0;
        while let Some(byte) = bytes.get(cursor) {
            if container_is_array
                && depth == 1
                && !byte.is_ascii_whitespace()
                && *byte != b','
                && *byte != b']'
                && matches!(segment, ConfigPathSegment::Index(index) if *index == element_index)
            {
                found = Some((cursor, cursor));
                break;
            }
            match byte {
                b',' if container_is_array && depth == 1 => element_index += 1,
                b'{' | b'[' => depth += 1,
                b'}' | b']' => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        break;
                    }
                }
                b'"' => {
                    let string_start = cursor;
                    cursor += 1;
                    loop {
                        match bytes.get(cursor)? {
                            b'\\' => cursor += 2,
                            b'"' => break,
                            _ => cursor += 1,
                        }
                    }
                    let string_end = cursor + 1;
                    let is_key = text.get(string_end..)?.trim_start().starts_with(':');
                    if depth == 1 && !container_is_array && is_key {
                        let name: String =
                            serde_json::from_str(text.get(string_start..string_end)?).ok()?;
                        if matches!(segment, ConfigPathSegment::Key(key) if key == &name) {
                            found = Some((string_start, string_end));
                            break;
                        }
                    }
                }
                _ => {}
            }
            cursor += 1;
        }
        let (start, end) = found?;
        position = start;
        search_start = end;
    }
    Some(position)
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

#[cfg(test)]
mod location_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn within_entry_location_skips_an_earlier_matching_list_and_keeps_fallback() {
        for pretty in [false, true] {
            let value = serde_json::json!({
                "rulebook":[{"markdown":"rulebook.md","rules":"rules.json"}],
                "marker_prefix":"K","check":{"files":["step"]},
                "evidence":{"ast_grep":"evidence/sgconfig.yml","kinds":{
                    "step":{"rule":"r","fields":{},
                        "select":{"question":{},"yes_at_or_above":0.7,"no_at_or_below":0.3,"shared":["within","missing"]},
                        "within":["parent","missing"]}
                }},"decision_model":{"model":"jev-latest"}
            });
            let text = if pretty {
                serde_json::to_string_pretty(&value).unwrap()
            } else {
                serde_json::to_string(&value).unwrap()
            };
            let location = ConfigLocation {
                file: "sunstack.lint.json".into(),
                directory: ".".into(),
            };
            let config = decode_config(&text, &location).unwrap();
            let position = text.rfind("\"missing\"").unwrap();
            let prefix = &text[..position];
            let line =
                u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count() + 1).unwrap();
            let column =
                u32::try_from(prefix.rsplit('\n').next().unwrap().chars().count() + 1).unwrap();
            let diagnostic = config_error(
                &config,
                &ConfigPath::keys(["evidence", "kinds", "step", "within"]).with_index(1),
                LintInvalidCode::Syntax,
                "undefined kind missing",
            );
            assert_eq!(
                (diagnostic.line, diagnostic.column),
                (Some(line), Some(column)),
                "pretty={pretty}"
            );
            let absent = config_error(
                &config,
                &ConfigPath::keys(["evidence", "kinds", "absent"]),
                LintInvalidCode::Syntax,
                "undefined kind absent",
            );
            assert_eq!((absent.line, absent.column), (Some(1), Some(1)));
        }
    }
    #[test]
    fn structured_array_indexes_skip_nested_values_and_keep_literal_keys() {
        for pretty in [false, true] {
            let value = serde_json::json!({"rows":[{"ignore":[1,2]},"escaped, \"value\"",{"a.b":{"body":[false,{"literal.key":"wanted"}]}}],"0":"object-key"});
            let text = if pretty {
                serde_json::to_string_pretty(&value).unwrap()
            } else {
                serde_json::to_string(&value).unwrap()
            };
            let indexed = ConfigPath::keys(["rows"])
                .with_index(2)
                .with_key("a.b")
                .with_key("body")
                .with_index(1)
                .with_key("literal.key");
            assert_eq!(
                config_field_position(&text, &indexed),
                text.find("\"literal.key\"")
            );
            assert_eq!(
                config_field_position(&text, &ConfigPath::keys(["rows"]).with_index(1)),
                text.find("\"escaped,")
            );
            assert_eq!(
                config_field_position(&text, &ConfigPath::keys(["rows"]).with_index(3)),
                None
            );
            assert_eq!(
                config_field_position(&text, &ConfigPath::keys(["rows", "2"])),
                None
            );
            assert_eq!(
                config_field_position(&text, &ConfigPath::keys(["0"])),
                text.find("\"0\"")
            );
        }
    }
    #[test]
    fn serde_paths_keep_map_names_and_sequence_indexes_distinct() {
        let text = r#"{"a.b":["valid",false]}"#;
        let mut decoder = serde_json::Deserializer::from_str(text);
        let result: Result<std::collections::BTreeMap<String, Vec<String>>, _> =
            serde_path_to_error::deserialize(&mut decoder);
        let error = result.unwrap_err();
        let path = path_fields(error.path());
        assert_eq!(
            path.segments(),
            &[
                ConfigPathSegment::Key("a.b".into()),
                ConfigPathSegment::Index(1)
            ]
        );
        assert_eq!(path.to_string(), "a.b.1");
        assert_eq!(config_field_position(text, &path), text.find("false"));
    }
}
