use pulldown_cmark::{Alignment, Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

pub(crate) struct OffsetProjection<'document> {
    text: &'document str,
    events: Vec<(Event<'document>, Range<usize>)>,
}

impl<'document> OffsetProjection<'document> {
    pub(crate) fn new(text: &'document str) -> Self {
        let options = Options::ENABLE_TABLES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
            | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS;
        Self {
            text,
            events: Parser::new_ext(text, options).into_offset_iter().collect(),
        }
    }

    pub(crate) fn prose(&self, range: Range<usize>, list_indent: bool) -> (String, String) {
        (
            self.visible(range.clone(), true, list_indent),
            self.visible(range, false, list_indent),
        )
    }

    pub(crate) fn code(&self, range: Range<usize>) -> (String, String) {
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
                Event::Code(text) if keep_code => {
                    let source = &self.text[span.clone()];
                    if source.contains("\r\n") {
                        // pulldown-cmark 0.13.4 emits two spaces for one CRLF in a code span.
                        output.push_str(&normalize_code_span(source));
                    } else {
                        output.push_str(text);
                    }
                }
                Event::Code(_) => {}
                Event::SoftBreak | Event::HardBreak => {
                    output.push('\n');
                    if list_indent {
                        let next_line = self.text[span.start..]
                            .find('\n')
                            .map_or(span.end, |offset| span.start + offset + 1);
                        output.extend(
                            self.text[next_line..]
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

fn normalize_code_span(source: &str) -> String {
    let content = source
        .trim_matches('`')
        .replace("\r\n", "\n")
        .replace(['\r', '\n'], " ");
    if content.starts_with(' ')
        && content.ends_with(' ')
        && content.chars().any(|character| character != ' ')
    {
        content[1..content.len() - 1].to_owned()
    } else {
        content
    }
}
