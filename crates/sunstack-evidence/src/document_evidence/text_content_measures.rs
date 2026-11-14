use crate::FieldUnavailable;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// Measure an entire HTML value before Markdown can split its block at blank lines.
pub fn count_html_content_words(
    raw: &str,
    settings: &crate::HtmlEvidenceSettings,
) -> Result<usize, FieldUnavailable> {
    use super::html_content::{HtmlAskingText, html_asking_text};
    match html_asking_text(raw, settings) {
        HtmlAskingText::Complete(text) => Ok(text.split_whitespace().count()),
        HtmlAskingText::Incomplete { .. } => Err(FieldUnavailable::Unreadable),
    }
}

/// Count readable Markdown content, keeping links' labels and code text, without markup.
pub fn count_text_content_words(
    raw: &str,
    settings: &crate::HtmlEvidenceSettings,
) -> Result<usize, FieldUnavailable> {
    use super::html_content::{HtmlAskingText, html_asking_text};
    let mut content = String::new();
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS;
    for event in Parser::new_ext(raw, options) {
        match event {
            Event::Text(text) | Event::Code(text) => content.push_str(&text),
            Event::Html(html) | Event::InlineHtml(html) => {
                content.push(' ');
                match html_asking_text(&html, settings) {
                    HtmlAskingText::Complete(text) => content.push_str(&text),
                    HtmlAskingText::Incomplete { .. } => return Err(FieldUnavailable::Unreadable),
                }
                content.push(' ');
            }
            Event::SoftBreak | Event::HardBreak | Event::Rule => {
                content.push(' ');
            }
            Event::End(
                TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::Item
                | TagEnd::TableCell
                | TagEnd::CodeBlock,
            )
            | Event::Start(Tag::TableRow | Tag::TableHead) => content.push(' '),
            _ => {}
        }
    }
    Ok(content.split_whitespace().count())
}

#[cfg(test)]
#[path = "text_content_measures_test.rs"]
mod tests;
