use crate::document_evidence::markdown_mapped_text::MappedMarkdownText;
use pulldown_cmark::{Alignment, Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

pub(crate) struct NormalizedMarkdown {
    text: String,
    removed_carriage_returns: Vec<RemovedCarriageReturn>,
}

struct RemovedCarriageReturn {
    original: usize,
    normalized: usize,
}

impl NormalizedMarkdown {
    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }

    pub(crate) fn original_range(&self, normalized: Range<usize>) -> Range<usize> {
        self.original_offset(normalized.start)..self.original_offset(normalized.end)
    }
    pub(crate) fn new(original: &str) -> Self {
        let mut text = String::with_capacity(original.len());
        let mut removed_carriage_returns = Vec::new();
        let mut characters = original.char_indices().peekable();
        while let Some((offset, character)) = characters.next() {
            if character == '\r' {
                if characters.peek().is_some_and(|(_, next)| *next == '\n') {
                    removed_carriage_returns.push(RemovedCarriageReturn {
                        original: offset,
                        normalized: text.len(),
                    });
                } else {
                    text.push('\n');
                }
            } else {
                text.push(character);
            }
        }
        Self {
            text,
            removed_carriage_returns,
        }
    }

    fn map_range(&self, original: Range<usize>) -> Range<usize> {
        let offset = |original| {
            original
                - self
                    .removed_carriage_returns
                    .partition_point(|removed| removed.original < original)
        };
        offset(original.start)..offset(original.end)
    }

    fn original_offset(&self, normalized: usize) -> usize {
        normalized
            + self
                .removed_carriage_returns
                .partition_point(|removed| removed.normalized < normalized)
    }

    fn append_text(&self, output: &mut MappedMarkdownText, text: &str, span: Range<usize>) {
        if self.text.get(span.clone()) != Some(text) {
            output.push(
                text,
                self.original_offset(span.start)..self.original_offset(span.end),
                false,
            );
            return;
        }
        let mut start = span.start;
        let first = self
            .removed_carriage_returns
            .partition_point(|removed| removed.normalized < span.start);
        let last = self
            .removed_carriage_returns
            .partition_point(|removed| removed.normalized < span.end);
        for removed in &self.removed_carriage_returns[first..last] {
            let newline = removed.normalized;
            output.push(
                &self.text[start..newline],
                self.original_offset(start)..removed.original,
                true,
            );
            output.push("\n", removed.original..removed.original + 2, false);
            start = newline + 1;
        }
        output.push(
            &self.text[start..span.end],
            self.original_offset(start)..self.original_offset(span.end),
            true,
        );
    }
}

pub(crate) struct OffsetProjection<'document> {
    source: &'document NormalizedMarkdown,
    events: Vec<(Event<'document>, Range<usize>)>,
}

impl<'document> OffsetProjection<'document> {
    pub(crate) fn new(source: &'document NormalizedMarkdown) -> Self {
        let options = Options::ENABLE_TABLES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
            | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS;
        Self {
            source,
            events: Parser::new_ext(&source.text, options)
                .into_offset_iter()
                .collect(),
        }
    }

    pub(crate) fn prose(&self, range: Range<usize>, list_indent: bool) -> (String, String) {
        (
            self.mapped_prose(range.clone(), true, list_indent).text,
            self.mapped_prose(range, false, list_indent).text,
        )
    }

    pub(crate) fn code(&self, range: Range<usize>) -> (String, String) {
        let text = self.mapped_code(range).text;
        (text.clone(), text)
    }

    pub(crate) fn mapped_code(&self, range: Range<usize>) -> MappedMarkdownText {
        let mut text = self.mapped_prose(range, true, false);
        text.trim_end();
        text
    }

    pub(crate) fn mapped_prose(
        &self,
        range: Range<usize>,
        keep_code: bool,
        list_indent: bool,
    ) -> MappedMarkdownText {
        self.mapped_visible(self.source.map_range(range), keep_code, list_indent)
    }

    fn mapped_visible(
        &self,
        range: Range<usize>,
        keep_code: bool,
        list_indent: bool,
    ) -> MappedMarkdownText {
        let mut output = MappedMarkdownText::default();
        let mut in_code_block = false;
        for (event, span) in &self.events {
            match event {
                Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
                Event::End(TagEnd::CodeBlock) => in_code_block = false,
                _ => {}
            }
            if span.start < range.start || span.end > range.end {
                continue;
            }
            match event {
                Event::Text(text)
                | Event::InlineMath(text)
                | Event::DisplayMath(text)
                | Event::FootnoteReference(text) => {
                    let escaped = !in_code_block
                        && text
                            .as_bytes()
                            .first()
                            .is_some_and(u8::is_ascii_punctuation)
                        && self.source.text[..span.start]
                            .bytes()
                            .rev()
                            .take_while(|byte| *byte == b'\\')
                            .count()
                            % 2
                            == 1;
                    if escaped && self.source.text.get(span.clone()) == Some(text.as_ref()) {
                        output.push(
                            &text[..1],
                            self.source.original_offset(span.start - 1)
                                ..self.source.original_offset(span.start + 1),
                            false,
                        );
                        self.source
                            .append_text(&mut output, &text[1..], span.start + 1..span.end);
                    } else {
                        self.source.append_text(&mut output, text, span.clone());
                    }
                }
                Event::Code(text) if keep_code => {
                    self.source.append_text(&mut output, text, span.clone())
                }
                Event::Code(_) => {}
                Event::SoftBreak | Event::HardBreak => {
                    output.push(
                        "\n",
                        self.source.original_offset(span.start)
                            ..self.source.original_offset(span.end),
                        false,
                    );
                    if list_indent {
                        let next_line = self.source.text[span.start..]
                            .find('\n')
                            .map_or(span.end, |offset| span.start + offset + 1);
                        let indent = self.source.text[next_line..]
                            .chars()
                            .take_while(|character| matches!(character, ' ' | '\t'))
                            .count();
                        self.source.append_text(
                            &mut output,
                            &self.source.text[next_line..next_line + indent],
                            next_line..next_line + indent,
                        );
                    }
                }
                Event::TaskListMarker(checked) => output.push(
                    if *checked { "[x] " } else { "[ ] " },
                    self.source.original_offset(span.start)..self.source.original_offset(span.end),
                    false,
                ),
                Event::Start(_)
                | Event::End(_)
                | Event::Html(_)
                | Event::InlineHtml(_)
                | Event::Rule => {}
            }
        }
        output
    }

    pub(crate) fn table(&self, range: Range<usize>) -> (String, String) {
        (
            self.mapped_table(range.clone(), true).text,
            self.mapped_table(range, false).text,
        )
    }

    pub(crate) fn mapped_table(&self, range: Range<usize>, keep_code: bool) -> MappedMarkdownText {
        let range = self.source.map_range(range);
        let mut output = MappedMarkdownText::default();
        let mut cells = Vec::new();
        let mut alignments = Vec::new();
        for (event, span) in &self.events {
            if span.start < range.start || span.end > range.end {
                continue;
            }
            match event {
                Event::Start(Tag::Table(columns)) => alignments = columns.clone(),
                Event::Start(Tag::TableHead | Tag::TableRow) => cells.clear(),
                Event::Start(Tag::TableCell) => {
                    let mut cell = self.mapped_visible(span.clone(), keep_code, false);
                    cell.trim();
                    cells.push(cell);
                }
                Event::End(TagEnd::TableHead) => {
                    append_table_row(
                        &mut output,
                        std::mem::take(&mut cells),
                        self.source.original_offset(span.start),
                    );
                    let delimiters = alignments
                        .iter()
                        .map(|alignment| match alignment {
                            Alignment::None => "---",
                            Alignment::Left => ":--",
                            Alignment::Right => "--:",
                            Alignment::Center => ":-:",
                        })
                        .map(|text| {
                            let mut cell = MappedMarkdownText::default();
                            let anchor = self.source.original_offset(span.end);
                            cell.push(text, anchor..anchor, false);
                            cell
                        })
                        .collect::<Vec<_>>();
                    append_table_row(
                        &mut output,
                        delimiters,
                        self.source.original_offset(span.end),
                    );
                }
                Event::End(TagEnd::TableRow) => append_table_row(
                    &mut output,
                    std::mem::take(&mut cells),
                    self.source.original_offset(span.start),
                ),
                _ => {}
            }
        }
        output
    }
}

fn append_table_row(
    output: &mut MappedMarkdownText,
    cells: Vec<MappedMarkdownText>,
    anchor: usize,
) {
    if !output.text.is_empty() {
        output.push("\n", anchor..anchor, false);
    }
    output.push("| ", anchor..anchor, false);
    for (index, cell) in cells.into_iter().enumerate() {
        if index > 0 {
            output.push(" | ", anchor..anchor, false);
        }
        output.append(cell);
    }
    output.push(" |", anchor..anchor, false);
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn rendered_matches_keep_original_lexemes_and_unicode_offsets() {
        let raw = "Élan **Use** &amp; \\* now\r\nnext\n";
        let normalized = NormalizedMarkdown::new(raw);
        let projection = OffsetProjection::new(&normalized);
        let mapped = projection.mapped_prose(0..raw.len(), false, false);
        assert_eq!(mapped.text, "Élan Use & * now\nnext");
        for (visible, source) in [("Use", "Use"), ("&", "&amp;"), ("*", "\\*"), ("\n", "\r\n")] {
            let start = mapped.text.find(visible).unwrap();
            let span = mapped.raw_range(start..start + visible.len()).unwrap();
            assert_eq!(&raw[span], source);
        }
    }

    #[test]
    fn distinct_visible_entity_scalars_share_one_atomic_raw_lexeme() {
        let raw = "&NotEqualTilde;\n";
        let normalized = NormalizedMarkdown::new(raw);
        let projection = OffsetProjection::new(&normalized);
        let mapped = projection.mapped_prose(0..raw.len(), false, false);
        let matches = regex::Regex::new(".")
            .unwrap()
            .find_iter(&mapped.text)
            .map(|found| (found.range(), mapped.raw_range(found.range()).unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 2);
        assert_ne!(matches[0].0, matches[1].0);
        assert_eq!(matches[0].1, 0..15);
        assert_eq!(matches[1].1, 0..15);
    }

    #[test]
    fn inline_code_is_excluded_but_fenced_text_keeps_raw_mapping() {
        let raw = "Use `hidden` visible\n\n```text\nUse fenced\n```\n";
        let normalized = NormalizedMarkdown::new(raw);
        let projection = OffsetProjection::new(&normalized);
        let mapped = projection.mapped_prose(0..raw.len(), false, false);
        assert_eq!(mapped.text, "Use  visibleUse fenced\n");
        let start = mapped.text.find("Use fenced").unwrap();
        let span = mapped.raw_range(start..start + 10).unwrap();
        assert_eq!(&raw[span], "Use fenced");
    }
}
