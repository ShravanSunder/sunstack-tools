use std::ops::Range;

#[derive(Default)]
pub(crate) struct MappedMarkdownText {
    pub(crate) text: String,
    segments: Vec<TextSegment>,
}

struct TextSegment {
    visible: Range<usize>,
    raw: Range<usize>,
    linear: bool,
}

impl MappedMarkdownText {
    pub(crate) fn push(&mut self, text: &str, raw: Range<usize>, linear: bool) {
        if text.is_empty() {
            return;
        }
        let start = self.text.len();
        self.text.push_str(text);
        self.segments.push(TextSegment {
            visible: start..self.text.len(),
            raw,
            linear,
        });
    }

    pub(crate) fn raw_range(&self, visible: Range<usize>) -> Option<Range<usize>> {
        if visible.is_empty() || visible.end > self.text.len() {
            return None;
        }
        let ranges = self
            .segments
            .iter()
            .filter_map(|segment| {
                let start = visible.start.max(segment.visible.start);
                let end = visible.end.min(segment.visible.end);
                (start < end).then(|| {
                    if segment.linear {
                        segment.raw.start + start - segment.visible.start
                            ..segment.raw.start + end - segment.visible.start
                    } else {
                        segment.raw.clone()
                    }
                })
            })
            .collect::<Vec<_>>();
        ranges
            .iter()
            .filter(|range| !range.is_empty())
            .cloned()
            .reduce(|left, right| left.start.min(right.start)..left.end.max(right.end))
            .or_else(|| ranges.first().cloned())
    }

    pub(crate) fn append(&mut self, mut other: Self) {
        let offset = self.text.len();
        self.text.push_str(&other.text);
        for segment in &mut other.segments {
            segment.visible.start += offset;
            segment.visible.end += offset;
        }
        self.segments.extend(other.segments);
    }

    pub(crate) fn trim(&mut self) {
        self.trim_end();
        let start = self.text.len() - self.text.trim_start().len();
        self.text.drain(..start);
        self.segments.retain(|segment| segment.visible.end > start);
        for segment in &mut self.segments {
            if segment.linear && segment.visible.start < start {
                segment.raw.start += start - segment.visible.start;
            }
            segment.visible.start = segment.visible.start.saturating_sub(start);
            segment.visible.end -= start;
        }
    }

    pub(crate) fn trim_end(&mut self) {
        let end = self.text.trim_end().len();
        self.text.truncate(end);
        self.segments.retain(|segment| segment.visible.start < end);
        if let Some(last) = self.segments.last_mut() {
            if last.linear {
                last.raw.end -= last.visible.end - end;
            }
            last.visible.end = end;
        }
    }
}
