use ast_grep_core::{
    Language, Pattern, PatternError,
    matcher::PatternBuilder,
    tree_sitter::{LanguageExt, StrDoc, TSLanguage},
};

// ast-grep exposes Markdown's block grammar; its inline grammar uses the same adapter API.
#[derive(Clone)]
struct MarkdownInline;
impl Language for MarkdownInline {
    fn kind_to_id(&self, kind: &str) -> u16 {
        self.get_ts_language().id_for_node_kind(kind, true)
    }
    fn field_to_id(&self, field: &str) -> Option<u16> {
        self.get_ts_language()
            .field_id_for_name(field)
            .map(|id| id.get())
    }
    fn build_pattern(&self, builder: &PatternBuilder) -> Result<Pattern, PatternError> {
        builder.build(|source| StrDoc::try_new(source, self.clone()))
    }
}
impl LanguageExt for MarkdownInline {
    fn get_ts_language(&self) -> TSLanguage {
        tree_sitter_md::INLINE_LANGUAGE.into()
    }
}

pub(crate) fn inside_code_span(text: &str, offset: usize) -> bool {
    MarkdownInline
        .ast_grep(text)
        .root()
        .dfs()
        .any(|node| node.kind() == "code_span" && node.range().contains(&offset))
}
