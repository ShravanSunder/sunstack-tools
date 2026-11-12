use crate::{EvidenceLanguage, EvidenceTargetResolution, FieldUnavailable};
use std::path::PathBuf;

pub(super) struct TargetReference {
    pub file: PathBuf,
    pub fragment: Option<String>,
}
#[derive(Clone, Copy)]
pub(super) enum TargetReferenceSyntax {
    HtmlUrl,
    MarkdownUrl,
    Literal,
}

impl TargetReferenceSyntax {
    pub(super) fn for_language(language: Option<EvidenceLanguage>) -> Self {
        match language {
            Some(EvidenceLanguage::Html) => Self::HtmlUrl,
            Some(EvidenceLanguage::Markdown) => Self::MarkdownUrl,
            _ => Self::Literal,
        }
    }
}

impl TargetReference {
    pub(super) fn uses_path(raw: &str, resolution: &EvidenceTargetResolution) -> bool {
        match resolution {
            EvidenceTargetResolution::Path => true,
            EvidenceTargetResolution::Skill => false,
            EvidenceTargetResolution::PathOrSkill => raw
                .split_once('#')
                .map_or(raw, |(file, _)| file)
                .ends_with(".md"),
        }
    }

    pub fn parse(raw: &str, syntax: TargetReferenceSyntax) -> Result<Self, FieldUnavailable> {
        let html = matches!(syntax, TargetReferenceSyntax::HtmlUrl);
        let reference = if html {
            crate::document_evidence::html_content::decode_entities(raw)
        } else {
            raw.to_owned()
        };
        let (file, fragment) = reference
            .split_once('#')
            .map_or((reference.as_str(), None), |(file, fragment)| {
                (file, Some(fragment))
            });
        let decode = |value: &str| {
            if !matches!(syntax, TargetReferenceSyntax::Literal) {
                decode_percent_utf8(value)
            } else {
                Ok(value.to_owned())
            }
        };
        Ok(Self {
            file: PathBuf::from(decode(file)?),
            fragment: fragment.map(decode).transpose()?,
        })
    }
}

#[cfg(test)]
#[path = "target_reference_test.rs"]
mod tests;

fn decode_percent_utf8(input: &str) -> Result<String, FieldUnavailable> {
    let raw = input.as_bytes();
    let mut decoded = Vec::with_capacity(raw.len());
    let mut position = 0;
    while position < raw.len() {
        if raw[position] == b'%'
            && let (Some(high), Some(low)) = (raw.get(position + 1), raw.get(position + 2))
            && let (Some(high), Some(low)) = (hex_value(*high), hex_value(*low))
        {
            decoded.push(high * 16 + low);
            position += 3;
        } else {
            decoded.push(raw[position]);
            position += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| FieldUnavailable::Unreadable)
}
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
