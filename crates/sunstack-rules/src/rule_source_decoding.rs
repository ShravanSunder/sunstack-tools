use crate::{
    DefinitionEntryLocation, DiagnosticList, ReportDiagnostic, RuleDiagnostic, RuleDiagnosticCode,
    RuleDiagnosticLocation, RuleSourceEntry, strict_decoding::decode_definition,
};
use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, Visitor},
};
use serde_json::value::RawValue;
use std::{collections::BTreeMap, fmt, path::Path};

struct RawEntries<'document>(Vec<(String, &'document RawValue)>);
#[derive(Deserialize)]
struct CheckBlocks<'a> {
    #[serde(borrow)]
    expect: Option<&'a RawValue>,
    #[serde(borrow)]
    ast_grep: Option<&'a RawValue>,
}

fn block_position(bytes: &[u8], block: &RawValue) -> Result<(usize, usize), DiagnosticList> {
    let start = block
        .get()
        .as_ptr()
        .addr()
        .checked_sub(bytes.as_ptr().addr())
        .filter(|start| *start <= bytes.len())
        .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError))?;
    let prefix = std::str::from_utf8(&bytes[..start])
        .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::MalformedJson))?;
    Ok((
        prefix.bytes().filter(|byte| *byte == b'\n').count() + 1,
        prefix
            .rsplit('\n')
            .next()
            .map_or(1, |tail| tail.chars().count() + 1),
    ))
}
impl<'de> Deserialize<'de> for RawEntries<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EntriesVisitor;
        impl<'de> Visitor<'de> for EntriesVisitor {
            type Value = RawEntries<'de>;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("one rules object with independently decoded entries")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry::<String, &'de RawValue>()? {
                    entries.push(entry);
                }
                Ok(RawEntries(entries))
            }
        }
        deserializer.deserialize_map(EntriesVisitor)
    }
}

pub(crate) struct DecodedRuleEntry {
    pub key: String,
    pub location: DefinitionEntryLocation,
    pub decoded: Result<RuleSourceEntry, DiagnosticList>,
}

pub(crate) fn decode_source_entries(
    bytes: &[u8],
    file: &Path,
) -> Result<Vec<DecodedRuleEntry>, DiagnosticList> {
    let malformed = |error: serde_json::Error| {
        DiagnosticList::from(
            RuleDiagnostic::new(RuleDiagnosticCode::MalformedJson).definition(
                file,
                error.line(),
                error.column(),
                vec![],
            ),
        )
    };
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let entries = RawEntries::deserialize(&mut deserializer).map_err(malformed)?;
    deserializer.end().map_err(malformed)?;
    let mut counts = BTreeMap::new();
    for (key, _) in &entries.0 {
        *counts.entry(key).or_insert(0usize) += 1;
    }
    let mut decoded = Vec::new();
    for (key, raw) in &entries.0 {
        let offset = raw
            .get()
            .as_ptr()
            .addr()
            .checked_sub(bytes.as_ptr().addr())
            .filter(|offset| *offset <= bytes.len())
            .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError))?;
        let prefix = std::str::from_utf8(&bytes[..offset])
            .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::MalformedJson))?;
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = prefix
            .rsplit('\n')
            .next()
            .map_or(1, |tail| tail.chars().count() + 1);
        let mut value = decode_definition::<RuleSourceEntry>(raw.get().as_bytes(), file, line)
            .map_err(|mut diagnostics| {
                for diagnostic in &mut diagnostics.items {
                    if let ReportDiagnostic::Rule(diagnostic) = diagnostic
                        && let RuleDiagnosticLocation::Definition {
                            line: error_line,
                            column: error_column,
                            ..
                        } = &mut diagnostic.location
                        && *error_line == line
                    {
                        *error_column += column - 1;
                    }
                }
                diagnostics
            });
        // Preserve the strict typed decoder's refusal, but recover the complete
        // dropped-key path from the original block for a located source error.
        if value.as_ref().err().is_some_and(|diagnostics| diagnostics.items.iter().any(|item|
            matches!(item, ReportDiagnostic::Rule(diagnostic) if diagnostic.code == RuleDiagnosticCode::UnknownField))) {
            let blocks: CheckBlocks<'_> = serde_json::from_str(raw.get()).map_err(malformed)?;
            if let Some(block) = blocks.ast_grep {
                let input = serde_json::from_str(block.get()).map_err(malformed)?;
                if let Ok(Some(mut path)) = crate::AstGrepRule::dropped_key(&input) {
                    path.insert(0, "ast_grep".into());
                    let (block_line, block_column) = block_position(bytes, block)?;
                    value = Err(RuleDiagnostic::new(RuleDiagnosticCode::UnknownField)
                        .definition(file, block_line, block_column, path).into());
                }
            }
        }
        if let Ok(entry) = &value {
            let check = match &entry.verdict {
                crate::RuleSourceVerdict::Expect { expect } => {
                    Some(crate::MechanicalCheck::Expect {
                        expect: expect.clone(),
                    })
                }
                crate::RuleSourceVerdict::AstGrep { ast_grep } => {
                    Some(crate::MechanicalCheck::AstGrep {
                        ast_grep: ast_grep.clone(),
                    })
                }
                _ => None,
            };
            if let Some(check) = check
                && let Err(mut diagnostics) = check.validate()
            {
                let blocks: CheckBlocks<'_> = serde_json::from_str(raw.get()).map_err(malformed)?;
                let block = blocks.ast_grep.or(blocks.expect).ok_or_else(|| {
                    DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError)
                })?;
                let (block_line, block_column) = block_position(bytes, block)?;
                crate::definition_admission::locate_at(
                    &mut diagnostics,
                    file,
                    block_line,
                    block_column,
                );
                value = Err(diagnostics);
            }
        }
        if counts.get(key).copied().unwrap_or_default() > 1 {
            let mut diagnostics = value.err().unwrap_or_default();
            diagnostics.extend(
                RuleDiagnostic::new(RuleDiagnosticCode::DuplicateField)
                    .definition(file, line, column, vec![])
                    .into(),
            );
            value = Err(diagnostics);
        }
        decoded.push(DecodedRuleEntry {
            key: key.clone(),
            location: DefinitionEntryLocation {
                file: file.into(),
                line,
                column,
            },
            decoded: value,
        });
    }
    Ok(decoded)
}
