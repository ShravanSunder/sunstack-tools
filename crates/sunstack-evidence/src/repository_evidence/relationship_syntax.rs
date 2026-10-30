//! Owned syntax facts. No tree node or filesystem handle escapes parsing.
use crate::*;
use ast_grep_language::{LanguageExt, SupportLang};
use std::ops::Range;

pub(crate) struct SyntaxDeclaration {
    pub name: String,
    pub qualified_name: Option<String>,
    pub unit: SelectedSourceUnit,
    pub default_export: bool,
}
pub(crate) struct SyntaxImport {
    pub local_name: String,
    pub imported_name: String,
    pub module_path: String,
    pub span: SourceSpan,
    pub unsupported: bool,
}
pub(crate) struct SyntaxCall {
    pub source_unit: SourceUnitKey,
    pub target_hint: String,
    pub span: SourceSpan,
    pub shadowed: bool,
    pub dynamic: bool,
}
pub(crate) struct ParsedSourceFile {
    pub file: FrozenRepositoryFile,
    pub declarations: Vec<SyntaxDeclaration>,
    pub imports: Vec<SyntaxImport>,
    pub calls: Vec<SyntaxCall>,
    pub conditional_modules: Vec<String>,
}

pub(crate) fn parse_relationship_file(
    file: &FrozenRepositoryFile,
    snapshot: &SnapshotId,
) -> EvidenceResult<Option<ParsedSourceFile>> {
    let SourceRef::RepositoryFile { path, .. } = &file.source else {
        return Err(EvidenceCode::InvalidSelector.into());
    };
    let language = match crate::repository_evidence::source_units::source_language(path) {
        Some(SourceLanguage::Rust) => "rust",
        Some(SourceLanguage::Typescript) => "typescript",
        Some(SourceLanguage::Tsx) => "tsx",
        _ => return Ok(None),
    };
    let parser: SupportLang = language
        .parse()
        .map_err(|_| EvidenceCode::UnsupportedLanguage)?;
    let tree = parser.ast_grep(&file.text);
    let units = extract_source_units(file, snapshot)?;
    let mut declarations = Vec::new();
    let mut imports = Vec::new();
    let mut shadows = Vec::<(String, Range<usize>)>::new();
    let mut conditional_modules = Vec::new();
    for node in tree.root().dfs() {
        let range = node.range();
        if let Some(unit) = units.iter().find(|unit| {
            unit.unit.key.byte_start == range.start && unit.unit.key.byte_end == range.end
        }) && let Some(name) = node.field("name")
        {
            declarations.push(SyntaxDeclaration {
                name: name.text().into_owned(),
                qualified_name: node
                    .ancestors()
                    .find(|ancestor| ancestor.kind() == "impl_item")
                    .and_then(|parent| parent.field("type"))
                    .map(|container| format!("{}::{}", container.text(), name.text())),
                unit: unit.clone(),
                default_export: node.ancestors().any(|ancestor| {
                    ancestor.kind() == "export_statement"
                        && ancestor.text().trim_start().starts_with("export default")
                }),
            });
        }
        match node.kind().as_ref() {
            "mod_item"
                if node
                    .prev_all()
                    .take_while(|previous| {
                        matches!(
                            previous.kind().as_ref(),
                            "attribute_item" | "line_comment" | "block_comment"
                        )
                    })
                    .any(|attribute| {
                        attribute.kind() == "attribute_item"
                            && (attribute.text().contains("cfg")
                                || attribute.text().contains("path"))
                    }) =>
            {
                if let Some(name) = node.field("name") {
                    conditional_modules.push(name.text().into_owned());
                }
            }
            "import_statement" => {
                let Some(module) = node.field("source") else {
                    continue;
                };
                let module_path = unquote(&module.text());
                let span = source_span(&file.source, &file.text, range);
                let mut found = false;
                for child in node.dfs() {
                    if child.kind() == "import_specifier" {
                        let Some(name) = child.field("name") else {
                            continue;
                        };
                        let alias = child.field("alias").unwrap_or_else(|| name.clone());
                        imports.push(SyntaxImport {
                            local_name: alias.text().into_owned(),
                            imported_name: name.text().into_owned(),
                            module_path: module_path.clone(),
                            span: span.clone(),
                            unsupported: false,
                        });
                        found = true;
                    }
                    if child.kind() == "import_clause" {
                        for name in child
                            .children()
                            .filter(|child| child.kind() == "identifier")
                        {
                            imports.push(SyntaxImport {
                                local_name: name.text().into_owned(),
                                imported_name: "default".into(),
                                module_path: module_path.clone(),
                                span: span.clone(),
                                unsupported: false,
                            });
                            found = true;
                        }
                    }
                }
                if node.dfs().any(|child| child.kind() == "namespace_import") || !found {
                    imports.push(SyntaxImport {
                        local_name: String::new(),
                        imported_name: String::new(),
                        module_path,
                        span,
                        unsupported: true,
                    });
                }
            }
            "use_declaration" => {
                let Some(argument) = node.field("argument") else {
                    continue;
                };
                let span = source_span(&file.source, &file.text, range);
                for path in expand_rust_use(&argument.text()) {
                    let (original, alias) = path
                        .split_once(" as ")
                        .map_or((path.as_str(), None), |(original, alias)| {
                            (original, Some(alias))
                        });
                    let (module_path, imported_name) =
                        original.rsplit_once("::").unwrap_or(("", original));
                    imports.push(SyntaxImport {
                        local_name: alias.unwrap_or(imported_name).trim().into(),
                        imported_name: imported_name.trim().into(),
                        module_path: module_path.trim().into(),
                        span: span.clone(),
                        unsupported: path.contains('*'),
                    });
                }
            }
            "variable_declarator"
            | "let_declaration"
            | "required_parameter"
            | "optional_parameter"
            | "parameter" => {
                if let Some(name) = node.field("name").or_else(|| node.field("pattern"))
                    && matches!(
                        name.kind().as_ref(),
                        "identifier" | "shorthand_property_identifier_pattern"
                    )
                {
                    let parameter = node.kind().contains("parameter");
                    let scope = node.ancestors().find(|ancestor| {
                        if parameter {
                            matches!(
                                ancestor.kind().as_ref(),
                                "function_item"
                                    | "function_declaration"
                                    | "method_definition"
                                    | "arrow_function"
                            )
                        } else {
                            matches!(
                                ancestor.kind().as_ref(),
                                "block" | "statement_block" | "program" | "source_file"
                            )
                        }
                    });
                    if let Some(scope) = scope {
                        let start = if parameter || language != "rust" {
                            scope.range().start
                        } else {
                            range.start
                        };
                        shadows.push((name.text().into_owned(), start..scope.range().end));
                    }
                }
            }
            _ => {}
        }
    }
    let mut calls = Vec::new();
    for node in tree
        .root()
        .dfs()
        .filter(|node| matches!(node.kind().as_ref(), "call_expression" | "macro_invocation"))
    {
        let range = node.range();
        let Some(source) = declarations
            .iter()
            .filter(|declaration| {
                declaration.unit.unit.key.byte_start <= range.start
                    && range.end <= declaration.unit.unit.key.byte_end
            })
            .min_by_key(|declaration| {
                declaration.unit.unit.key.byte_end - declaration.unit.unit.key.byte_start
            })
        else {
            continue;
        };
        let target = node.field("function").or_else(|| node.field("macro"));
        let Some(target) = target else {
            continue;
        };
        let hint = target.text().into_owned();
        let dynamic = !matches!(target.kind().as_ref(), "identifier" | "scoped_identifier")
            || node.kind() == "macro_invocation";
        let shadowed = shadows
            .iter()
            .any(|(name, scope)| name == &hint && scope.contains(&range.start));
        calls.push(SyntaxCall {
            source_unit: source.unit.unit.key.clone(),
            target_hint: hint,
            span: source_span(&file.source, &file.text, range),
            shadowed,
            dynamic,
        });
    }
    Ok(Some(ParsedSourceFile {
        file: file.clone(),
        declarations,
        imports,
        calls,
        conditional_modules,
    }))
}
fn unquote(text: &str) -> String {
    text.trim_matches(['\'', '"', '`']).into()
}
fn expand_rust_use(text: &str) -> Vec<String> {
    let Some(open) = text.find('{') else {
        return vec![text.trim().into()];
    };
    let Some(close) = text.rfind('}') else {
        return vec![text.trim().into()];
    };
    let prefix = text[..open].trim_end_matches(':');
    let mut depth = 0usize;
    let mut start = open + 1;
    let mut parts = Vec::new();
    for (offset, character) in text[open + 1..close].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(text[start..open + 1 + offset].trim());
                start = open + 2 + offset;
            }
            _ => {}
        }
    }
    parts.push(text[start..close].trim());
    parts
        .into_iter()
        .filter(|part| !part.is_empty())
        .flat_map(|part| expand_rust_use(&format!("{prefix}::{part}")))
        .collect()
}
