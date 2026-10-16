use crate::{relationship_syntax::*, *};
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy)]
pub(crate) struct ResolvedEndpoint<'source> {
    pub declaration: &'source SyntaxDeclaration,
    pub import: Option<&'source SyntaxImport>,
    pub mapping: Option<&'source PackageSourceBinding>,
}
pub(crate) fn call_endpoints<'source>(
    file: &'source ParsedSourceFile,
    hint: &str,
    files: &'source [ParsedSourceFile],
    bindings: &'source [PackageSourceBinding],
) -> Vec<ResolvedEndpoint<'source>> {
    let mut endpoints = file
        .declarations
        .iter()
        .filter(|declaration| {
            declaration.qualified_name.as_deref() == Some(hint)
                || (declaration.name == hint
                    && !matches!(declaration.unit.unit.kind, SourceUnitKind::Method { .. }))
        })
        .map(|declaration| ResolvedEndpoint {
            declaration,
            import: None,
            mapping: None,
        })
        .collect::<Vec<_>>();
    for import in file
        .imports
        .iter()
        .filter(|import| import.local_name == hint && !import.unsupported)
    {
        endpoints.extend(import_endpoints(file, import, files, bindings));
    }
    if let Some((module, name)) = hint.rsplit_once("::") {
        let synthetic = SyntaxImport {
            local_name: name.into(),
            imported_name: name.into(),
            module_path: module.into(),
            span: file
                .declarations
                .first()
                .map(|declaration| declaration.unit.unit.span.clone())
                .unwrap_or_else(|| source_span(&file.file.source, &file.file.text, 0..0)),
            unsupported: false,
        };
        for (target, mapping) in module_files(file, &synthetic, files, bindings) {
            endpoints.extend(
                target
                    .declarations
                    .iter()
                    .filter(|declaration| declaration.name == name)
                    .map(|declaration| ResolvedEndpoint {
                        declaration,
                        import: None,
                        mapping,
                    }),
            );
        }
    }
    endpoints
}
pub(crate) fn import_endpoints<'source>(
    file: &'source ParsedSourceFile,
    import: &'source SyntaxImport,
    files: &'source [ParsedSourceFile],
    bindings: &'source [PackageSourceBinding],
) -> Vec<ResolvedEndpoint<'source>> {
    if import.unsupported {
        return Vec::new();
    }
    module_files(file, import, files, bindings)
        .into_iter()
        .flat_map(|(target, mapping)| {
            target
                .declarations
                .iter()
                .filter(|declaration| {
                    if import.imported_name == "default" {
                        declaration.default_export
                    } else {
                        declaration.name == import.imported_name
                    }
                })
                .map(move |declaration| ResolvedEndpoint {
                    declaration,
                    import: Some(import),
                    mapping,
                })
        })
        .collect()
}
fn module_files<'source>(
    file: &ParsedSourceFile,
    import: &SyntaxImport,
    files: &'source [ParsedSourceFile],
    bindings: &'source [PackageSourceBinding],
) -> Vec<(
    &'source ParsedSourceFile,
    Option<&'source PackageSourceBinding>,
)> {
    let SourceRef::RepositoryFile {
        repository, path, ..
    } = &file.file.source
    else {
        return vec![];
    };
    let rust = path.extension().is_some_and(|extension| extension == "rs");
    let language = if rust {
        "rust"
    } else if path.extension().is_some_and(|extension| extension == "tsx") {
        "tsx"
    } else {
        "typescript"
    };
    let module = import.module_path.as_str();
    if rust
        && module.split("::").any(|segment| {
            file.conditional_modules
                .iter()
                .any(|conditional| conditional == segment)
        })
    {
        return Vec::new();
    }
    let mut targets = Vec::new();
    // Prefix matches require a module boundary. Overlapping explicit mappings
    // remain candidates; their order never chooses a winner.
    for binding in bindings.iter().filter(|binding| {
        &binding.importing_repository_id == repository && binding.language == language
    }) {
        let suffix = if module == binding.module_prefix {
            Some("")
        } else {
            module
                .strip_prefix(&binding.module_prefix)
                .and_then(|suffix| suffix.strip_prefix(if rust { "::" } else { "/" }))
        };
        if let Some(suffix) = suffix {
            let path = binding.target_path_prefix.join(if rust {
                suffix.replace("::", "/")
            } else {
                suffix.into()
            });
            targets.push((
                &binding.target_repository_id,
                normalize_path(&path),
                Some(binding),
            ));
        }
    }
    if targets.is_empty() {
        let parent = path.parent().unwrap_or(Path::new(""));
        let relative = if rust {
            if let Some(module) = module.strip_prefix("crate::") {
                Some(PathBuf::from("src").join(module.replace("::", "/")))
            } else if module == "crate" {
                Some(PathBuf::from("src/lib"))
            } else if let Some(module) = module.strip_prefix("self::") {
                Some(parent.join(module.replace("::", "/")))
            } else if let Some(module) = module.strip_prefix("super::") {
                Some(
                    parent
                        .parent()
                        .unwrap_or(Path::new(""))
                        .join(module.replace("::", "/")),
                )
            } else {
                Some(parent.join(module.replace("::", "/")))
            }
        } else if module.starts_with('.') {
            Some(parent.join(module))
        } else {
            None
        };
        if let Some(path) = relative {
            targets.push((repository, normalize_path(&path), None));
        }
    }
    let mut found = Vec::new();
    for (target_repository, target_path, mapping) in targets {
        let Some(target_path) = target_path else {
            continue;
        };
        let candidates = path_candidates(&target_path, rust);
        for target in files {
            let SourceRef::RepositoryFile {
                repository: candidate_repository,
                path: candidate_path,
                revision,
                ..
            } = &target.file.source
            else {
                continue;
            };
            if candidate_repository != target_repository || !candidates.contains(candidate_path) {
                continue;
            }
            if candidate_repository == repository {
                let SourceRef::RepositoryFile {
                    revision: source_revision,
                    ..
                } = &file.file.source
                else {
                    continue;
                };
                if revision != source_revision {
                    continue;
                }
            }
            found.push((target, mapping));
        }
    }
    found
}
pub(crate) fn endpoint_is_ambiguous(
    endpoint: &ResolvedEndpoint<'_>,
    candidates: &[ResolvedEndpoint<'_>],
) -> bool {
    let key = &endpoint.declaration.unit.unit.key;
    candidates
        .iter()
        .filter(|candidate| {
            let candidate_key = &candidate.declaration.unit.unit.key;
            candidate_key.repository == key.repository && candidate_key.revision == key.revision
        })
        .count()
        > 1
}
fn normalize_path(path: &Path) -> Option<PathBuf> {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => result.push(segment),
            Component::CurDir => {}
            Component::ParentDir if result.pop() => {}
            _ => return None,
        }
    }
    Some(result)
}
fn path_candidates(path: &Path, rust: bool) -> Vec<PathBuf> {
    let mut candidates = vec![path.into()];
    if path.extension().is_none() {
        if rust {
            candidates.extend([path.with_extension("rs"), path.join("mod.rs")]);
        } else {
            candidates.extend([
                path.with_extension("ts"),
                path.with_extension("tsx"),
                path.join("index.ts"),
                path.join("index.tsx"),
            ]);
        }
    }
    candidates
}
