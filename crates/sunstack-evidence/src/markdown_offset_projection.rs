use pulldown_cmark::{Alignment, Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

pub(crate) struct NormalizedMarkdown {
    text: String,
    removed_carriage_returns: Vec<usize>,
}

impl NormalizedMarkdown {
    pub(crate) fn new(original: &str) -> Self {
        let mut text = String::with_capacity(original.len());
        let mut removed_carriage_returns = Vec::new();
        let mut characters = original.char_indices().peekable();
        while let Some((offset, character)) = characters.next() {
            if character == '\r' {
                if characters.peek().is_some_and(|(_, next)| *next == '\n') {
                    removed_carriage_returns.push(offset);
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
                    .partition_point(|removed| *removed < original)
        };
        offset(original.start)..offset(original.end)
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
        let range = self.source.map_range(range);
        (
            self.visible(range.clone(), true, list_indent),
            self.visible(range, false, list_indent),
        )
    }

    pub(crate) fn code(&self, range: Range<usize>) -> (String, String) {
        let range = self.source.map_range(range);
        let text = self.visible(range, true, false);
        let text = text.trim_end().to_owned();
        (text.clone(), text)
    }

    fn visible(&self, range: Range<usize>, keep_code: bool, list_indent: bool) -> String {
        let mut output = String::new();
        for (event, span) in &self.events {
            if span.start < range.start || span.end > range.end {
                continue;
            }
            match event {
                Event::Text(text)
                | Event::InlineMath(text)
                | Event::DisplayMath(text)
                | Event::FootnoteReference(text) => output.push_str(text),
                Event::Code(text) if keep_code => output.push_str(text),
                Event::Code(_) => {}
                Event::SoftBreak | Event::HardBreak => {
                    output.push('\n');
                    if list_indent {
                        let next_line = self.source.text[span.start..]
                            .find('\n')
                            .map_or(span.end, |offset| span.start + offset + 1);
                        output.extend(
                            self.source.text[next_line..]
                                .chars()
                                .take_while(|character| matches!(character, ' ' | '\t')),
                        );
                    }
                }
                Event::TaskListMarker(checked) => {
                    output.push_str(if *checked { "[x] " } else { "[ ] " })
                }
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
        let range = self.source.map_range(range);
        let render = |keep_code| {
            let mut output = String::new();
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
                        cells.push(
                            self.visible(span.clone(), keep_code, false)
                                .trim()
                                .to_owned(),
                        );
                    }
                    Event::End(TagEnd::TableHead) => {
                        append_table_row(&mut output, &cells);
                        let delimiters = alignments
                            .iter()
                            .map(|alignment| match alignment {
                                Alignment::None => "---",
                                Alignment::Left => ":--",
                                Alignment::Right => "--:",
                                Alignment::Center => ":-:",
                            })
                            .map(str::to_owned)
                            .collect::<Vec<_>>();
                        append_table_row(&mut output, &delimiters);
                    }
                    Event::End(TagEnd::TableRow) => append_table_row(&mut output, &cells),
                    _ => {}
                }
            }
            output
        };
        (render(true), render(false))
    }
}

fn append_table_row(output: &mut String, cells: &[String]) {
    if !output.is_empty() {
        output.push('\n');
    }
    output.push_str("| ");
    output.push_str(&cells.join(" | "));
    output.push_str(" |");
}
