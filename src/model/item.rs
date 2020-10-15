use std::ops::Range;
use std::path::PathBuf;

use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::ListItem;

use crate::model::Printable;
use crate::rg::de::{ArbitraryData, RgMessage, RgMessageKind, SubMatch};
use crate::ui::render::{ToListItem, UiItemContext};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SubItem {
    pub index: usize,
    pub sub_match: SubMatch,
    pub should_replace: bool,
}

impl SubItem {
    pub fn new(index: usize, sub_match: SubMatch) -> SubItem {
        SubItem {
            index,
            sub_match,
            should_replace: true,
        }
    }
}

impl SubItem {
    pub fn line_count(&self) -> usize {
        self.sub_match.text.lossy_utf8().lines().count()
    }

    /// A SubItem contains the "match". A match _may_ be over multiple lines, but there will only ever
    /// be a single span on each line. So this returns a list of "lines": one span for each line.
    pub fn to_span_lines(&self, ctx: &UiItemContext, is_item_selected: bool) -> Vec<Span> {
        let mut s = Style::default();
        if ctx.replacement_text.is_some() {
            if self.should_replace {
                s = s.fg(Color::Red).add_modifier(Modifier::CROSSED_OUT);
            } else {
                s = s.fg(Color::DarkGray);
            }
        } else {
            if is_item_selected && ctx.ui_list_state.selected_submatch() == self.index {
                if self.should_replace {
                    s = s.fg(Color::Black).bg(Color::Yellow);
                } else {
                    s = s.fg(Color::Yellow).bg(Color::DarkGray);
                }
            } else {
                if self.should_replace {
                    s = s.fg(Color::Black).bg(Color::Red);
                } else {
                    s = s.fg(Color::Red).bg(Color::DarkGray);
                }
            }
        }

        self.sub_match
            .text
            .to_printable(ctx.printable_style)
            .lines()
            .map(|line| Span::styled(line.to_string(), s))
            .collect()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Item {
    pub index: usize,
    pub kind: RgMessageKind,
    rg_message: RgMessage,

    sub_items: Vec<SubItem>,
}

impl Item {
    pub fn new(index: usize, rg_message: RgMessage) -> Item {
        let kind = match &rg_message {
            RgMessage::Begin { .. } => RgMessageKind::Begin,
            RgMessage::End { .. } => RgMessageKind::End,
            RgMessage::Match { .. } => RgMessageKind::Match,
            RgMessage::Context { .. } => RgMessageKind::Context,
            RgMessage::Summary { .. } => RgMessageKind::Summary,
        };

        let sub_items = match &rg_message {
            RgMessage::Match { submatches, .. } => submatches
                .iter()
                .enumerate()
                .map(|(i, s)| SubItem::new(i, s.clone()))
                .collect(),
            _ => vec![],
        };

        Item {
            index,
            kind,
            rg_message,
            sub_items,
        }
    }

    pub fn get_should_replace(&self, idx: usize) -> bool {
        self.sub_items[idx].should_replace
    }

    pub fn set_should_replace(&mut self, idx: usize, should_replace: bool) {
        self.sub_items[idx].should_replace = should_replace
    }

    pub fn get_should_replace_all(&self) -> bool {
        self.sub_items.iter().all(|s| s.should_replace)
    }

    pub fn set_should_replace_all(&mut self, should_replace: bool) {
        for sub_item in &mut self.sub_items {
            sub_item.should_replace = should_replace;
        }
    }

    pub fn is_selectable(&self) -> bool {
        matches!(self.kind, RgMessageKind::Begin | RgMessageKind::Match)
    }

    pub fn line_number(&self) -> Option<&usize> {
        match &self.rg_message {
            RgMessage::Context { line_number, .. } => line_number.as_ref(),
            RgMessage::Match { line_number, .. } => line_number.as_ref(),
            _ => None,
        }
    }

    pub fn offset(&self) -> Option<usize> {
        match &self.rg_message {
            RgMessage::End { binary_offset, .. } => *binary_offset,
            RgMessage::Match {
                absolute_offset, ..
            } => Some(*absolute_offset),
            _ => None,
        }
    }

    pub fn replace_count(&self) -> usize {
        self.sub_items.iter().filter(|s| s.should_replace).count()
    }

    pub fn sub_items(&self) -> &[SubItem] {
        &self.sub_items
    }

    pub fn path(&self) -> Option<&ArbitraryData> {
        match &self.rg_message {
            RgMessage::Begin { path, .. } => Some(path),
            RgMessage::Match { path, .. } => Some(path),
            RgMessage::Context { path, .. } => Some(path),
            RgMessage::End { path, .. } => Some(path),
            RgMessage::Summary { .. } => None,
        }
    }

    pub fn path_buf(&self) -> Option<PathBuf> {
        self.path().and_then(|data| data.to_path_buf().ok())
    }

    fn line_number_to_span<'a>(mut style: Style, is_selected: bool, n: usize) -> Span<'a> {
        if !is_selected {
            style = style.fg(Color::DarkGray);
        }

        Span::styled(format!("{}:", n), style)
    }

    pub fn line_count(&self) -> usize {
        match &self.rg_message {
            RgMessage::Begin { .. } | RgMessage::End { .. } => 1,
            RgMessage::Match { lines, .. } | RgMessage::Context { lines, .. } => lines.lossy_utf8().lines().count(),
            RgMessage::Summary { .. } => 0
        }
    }
}

impl ToListItem for Item {
    fn to_list_item(&self, ctx: &UiItemContext) -> Vec<ListItem> {
        let is_replacing = ctx.replacement_text.is_some();
        let is_selected = ctx.ui_list_state.selected_item() == self.index;

        let mut base_style = Style::default();
        if !is_replacing && is_selected {
            base_style = base_style.fg(Color::Yellow);
        }

        match &self.rg_message {
            RgMessage::Begin { .. } => vec![ListItem::new(Span::styled(
                format!("{}", self.path_buf().unwrap().display()),
                if !is_replacing && is_selected {
                    base_style.fg(Color::Black).bg(Color::Yellow)
                } else {
                    base_style.fg(Color::Magenta)
                },
            ))],

            RgMessage::Context {
                lines, line_number, ..
            } => {
                let mut list_items = vec![];
                for (i, line) in lines.lossy_utf8().lines().enumerate() {
                    let mut spans = vec![];
                    if i == 0 {
                        if let Some(n) = line_number {
                            spans.push(Item::line_number_to_span(base_style, is_selected, *n));
                        }
                    }

                    spans.push(Span::styled(line.to_string(), base_style));
                    list_items.push(ListItem::new(Spans::from(spans)));
                }

                list_items
            }

            RgMessage::Match {
                lines, line_number, ..
            } => {
                let mut line_number = line_number.clone();

                // Read the lines as bytes since we convert each span to a string when it's created.
                // This ensures our alignments are correct.
                let lines_bytes = lines.to_vec();
                let replacement_span = ctx.replacement_text.map(|r| {
                    Span::styled(
                        r.to_printable(ctx.printable_style),
                        base_style.fg(Color::Green),
                    )
                });


                let mut list_items = vec![];
                let mut spans = vec![]; // re-used multiple times

                macro_rules! push_line_number_span {
                    // pushes a span to `spans` which contains the line number
                    () => {
                        if let Some(n) = line_number {
                            spans.push(Item::line_number_to_span(base_style, is_selected, n));
                        }
                    };
                    // increments the current line number first, then does the above
                    (++) => {
                        line_number.as_mut().map(|n| *n += 1);
                        push_line_number_span!();
                    }
                };

                let mut offset = 0;
                for (idx, sub_item) in self.sub_items.iter().enumerate() {
                    let Range { start, end } = sub_item.sub_match.range;

                    if idx == 0 {
                        push_line_number_span!();
                    }

                    // Text in between start (or last SubMatch) and this SubMatch.
                    let leading = offset..start;
                    #[allow(clippy::len_zero)]
                    if leading.len() > 0 {
                        spans.push(Span::styled(
                            String::from_utf8_lossy(&lines_bytes[leading]).to_string(),
                            base_style,
                        ));
                    }

                    // Match text, also may contain any leading line numbers and text from before.
                    let span_lines = sub_item.to_span_lines(ctx, is_selected);
                    let span_lines_len = span_lines.len();
                    for (i, span) in span_lines.into_iter().enumerate() {
                        if i > 0 {
                            // HACK: this just increments the line number for each match.
                            // surely there's a better way of doing this.
                            push_line_number_span!(++);
                        }

                        spans.push(span);

                        // Don't create a list item for the last line in the submatch, since we will append the replacement
                        // text there and any remaining non-match text from the line.
                        if i != span_lines_len - 1 {
                            list_items.push(ListItem::new(Spans::from(
                                spans.drain(..).collect::<Vec<_>>(),
                            )));
                        }
                    }

                    // Replacement text.
                    if sub_item.should_replace {
                        if let Some(span) = replacement_span.as_ref() {
                            spans.push(span.clone());
                        }
                    }

                    offset = end;
                }

                // Text after the last SubMatch and before the end of the line.
                let trailing = offset..lines_bytes.len();
                #[allow(clippy::len_zero)]
                if trailing.len() > 0 {
                    spans.push(Span::styled(
                        String::from_utf8_lossy(&lines_bytes[trailing]).to_string(),
                        base_style,
                    ));
                }

                list_items.push(ListItem::new(Spans::from(spans)));
                list_items
            }
            RgMessage::End { .. } => vec![ListItem::new(Span::from(""))],
            RgMessage::Summary { elapsed_total, .. } => vec![ListItem::new(Span::styled(
                format!("Search duration: {}", elapsed_total.human),
                base_style,
            ))],
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use pretty_assertions::assert_eq;
    use tui::style::{Color, Modifier, Style};
    use tui::text::{Span, Spans};

    use crate::model::*;
    use crate::rg::de::test_utilities::*;
    use crate::rg::de::*;

    pub fn new_item(raw_json: &str) -> Item {
        Item::new(RgMessage::from_str(raw_json))
    }

    #[test]
    fn item_kind_matches_rg_message_kind() {
        assert_eq!(new_item(RG_JSON_BEGIN).kind, RgMessageKind::Begin);
        assert_eq!(new_item(RG_JSON_MATCH).kind, RgMessageKind::Match);
        assert_eq!(new_item(RG_JSON_CONTEXT).kind, RgMessageKind::Context);
        assert_eq!(new_item(RG_JSON_END).kind, RgMessageKind::End);
        assert_eq!(new_item(RG_JSON_SUMMARY).kind, RgMessageKind::Summary);
    }

    #[test]
    fn only_match_and_begin_are_selectable() {
        assert_eq!(new_item(RG_JSON_BEGIN).is_selectable(), true);
        assert_eq!(new_item(RG_JSON_MATCH).is_selectable(), true);
        assert_eq!(new_item(RG_JSON_CONTEXT).is_selectable(), false);
        assert_eq!(new_item(RG_JSON_END).is_selectable(), false);
        assert_eq!(new_item(RG_JSON_SUMMARY).is_selectable(), false);
    }

    #[test]
    fn match_count() {
        assert_eq!(new_item(RG_JSON_BEGIN).sub_items().len(), 0);
        assert_eq!(new_item(RG_JSON_MATCH).sub_items().len(), 2);
        assert_eq!(new_item(RG_JSON_CONTEXT).sub_items().len(), 0);
        assert_eq!(new_item(RG_JSON_END).sub_items().len(), 0);
        assert_eq!(new_item(RG_JSON_SUMMARY).sub_items().len(), 0);
    }

    #[test]
    fn sub_items() {
        assert_eq!(new_item(RG_JSON_BEGIN).sub_items(), &[]);
        assert_eq!(
            new_item(RG_JSON_MATCH).sub_items(),
            &[
                SubItem::new(SubMatch::new_text("Item", 4..8)),
                SubItem::new(SubMatch::new_text("rg_msg", 14..20))
            ]
        );
        assert_eq!(new_item(RG_JSON_CONTEXT).sub_items(), &[]);
        assert_eq!(new_item(RG_JSON_END).sub_items(), &[]);
        assert_eq!(new_item(RG_JSON_SUMMARY).sub_items(), &[]);
    }

    #[test]
    fn offset() {
        assert_eq!(new_item(RG_JSON_BEGIN).offset(), None);
        assert_eq!(new_item(RG_JSON_MATCH).offset(), Some(5522));
        assert_eq!(new_item(RG_JSON_CONTEXT).offset(), None);
        assert_eq!(new_item(RG_JSON_END).offset(), None);
        assert_eq!(new_item(RG_JSON_SUMMARY).offset(), None);
    }

    #[test]
    fn binary_offset() {
        let item = new_item(
            r#"{"type":"end","data":{"path":{"text":"src/model/item.rs"},"binary_offset":1234,"stats":{"elapsed":{"secs":0,"nanos":97924,"human":"0.000098s"},"searches":1,"searches_with_match":1,"bytes_searched":5956,"bytes_printed":674,"matched_lines":2,"matches":2}}}"#,
        );
        assert_eq!(item.offset(), Some(1234));
    }

    #[test]
    fn path_with_text() {
        let path = PathBuf::from("src/model/item.rs");
        assert_eq!(new_item(RG_JSON_BEGIN).path_buf().as_ref(), Some(&path));
        assert_eq!(new_item(RG_JSON_MATCH).path_buf().as_ref(), Some(&path));
        assert_eq!(new_item(RG_JSON_CONTEXT).path_buf().as_ref(), Some(&path));
        assert_eq!(new_item(RG_JSON_END).path_buf().as_ref(), Some(&path));
        assert_eq!(new_item(RG_JSON_SUMMARY).path_buf().as_ref(), None);
    }

    // TODO: write a similar test for Windows systems
    #[test]
    #[cfg(unix)]
    fn path_with_base64() {
        use crate::rg::de::test_utilities::RgMessageBuilder;
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        // Here, the values 0x66 and 0x6f correspond to 'f' and 'o'
        // respectively. The value 0x80 is a lone continuation byte, invalid
        // in a UTF-8 sequence.
        let invalid_utf8_name_bytes = [0x66, 0x6f, 0x80, 0x6f];
        let invalid_utf8_name = OsStr::from_bytes(&invalid_utf8_name_bytes[..]);
        let invalid_utf8_path = PathBuf::from(invalid_utf8_name);

        let new_item_path_base64 = |kind| {
            Item::new(
                RgMessageBuilder::new(kind)
                    .with_path_base64(base64::encode(&invalid_utf8_name_bytes))
                    .with_lines_text("foo bar baz")
                    .with_submatches(vec![SubMatch::new_text("foo", 0..3)])
                    .with_stats(Stats::new())
                    .with_elapsed_total(Duration::new())
                    .with_offset(0)
                    .build(),
            )
        };

        assert_eq!(
            new_item_path_base64(RgMessageKind::Begin)
                .path_buf()
                .as_ref(),
            Some(&invalid_utf8_path)
        );
        assert_eq!(
            new_item_path_base64(RgMessageKind::Match)
                .path_buf()
                .as_ref(),
            Some(&invalid_utf8_path)
        );
        assert_eq!(
            new_item_path_base64(RgMessageKind::Context)
                .path_buf()
                .as_ref(),
            Some(&invalid_utf8_path)
        );
        assert_eq!(
            new_item_path_base64(RgMessageKind::End).path_buf().as_ref(),
            Some(&invalid_utf8_path)
        );
        assert_eq!(
            new_item_path_base64(RgMessageKind::Summary)
                .path_buf()
                .as_ref(),
            None
        );
    }

    #[test]
    fn to_span_with_text() {
        let s = Style::default();
        assert_eq!(
            new_item(RG_JSON_BEGIN).to_spans(None, None, PrintableStyle::Common),
            Spans::from(vec![Span::styled(
                "src/model/item.rs",
                s.fg(Color::Magenta)
            )])
        );
        assert_eq!(
            new_item(RG_JSON_MATCH).to_spans(None, None, PrintableStyle::Common),
            Spans::from(vec![
                Span::styled("197:", s.fg(Color::DarkGray)),
                Span::styled("    ", s),
                Span::styled("Item", s.bg(Color::Red).fg(Color::Black)),
                Span::styled("::new(", s),
                Span::styled("rg_msg", s.fg(Color::Black).bg(Color::Red)),
                Span::styled(")\n", s),
            ])
        );
        assert_eq!(
            new_item(RG_JSON_CONTEXT).to_spans(None, None, PrintableStyle::Common),
            Spans::from(vec![
                Span::styled("198:", s.fg(Color::DarkGray)),
                Span::styled("  }\n", s),
            ])
        );
        assert_eq!(
            new_item(RG_JSON_END).to_spans(None, None, PrintableStyle::Common),
            Spans::from("")
        );
        assert_eq!(
            new_item(RG_JSON_SUMMARY).to_spans(None, None, PrintableStyle::Common),
            Spans::from("Search duration: 0.013911s")
        );
    }

    #[test]
    fn to_span_with_text_replacement() {
        let s = Style::default();
        let replacement = "foobar";
        assert_eq!(
            new_item(RG_JSON_BEGIN).to_spans(Some(replacement), None, PrintableStyle::Common),
            Spans::from(vec![Span::styled(
                "src/model/item.rs",
                s.fg(Color::Magenta)
            )])
        );
        assert_eq!(
            new_item(RG_JSON_MATCH).to_spans(Some(replacement), None, PrintableStyle::Common),
            Spans::from(vec![
                Span::styled("197:", s.fg(Color::DarkGray)),
                Span::styled("    ", s),
                Span::styled("Item", s.fg(Color::Red).add_modifier(Modifier::CROSSED_OUT)),
                Span::styled("foobar", s.fg(Color::Green)),
                Span::styled("::new(", s),
                Span::styled(
                    "rg_msg",
                    s.fg(Color::Red).add_modifier(Modifier::CROSSED_OUT)
                ),
                Span::styled("foobar", s.fg(Color::Green)),
                Span::styled(")\n", s),
            ])
        );
        assert_eq!(
            new_item(RG_JSON_CONTEXT).to_spans(Some(replacement), None, PrintableStyle::Common),
            Spans::from(vec![
                Span::styled("198:", s.fg(Color::DarkGray)),
                Span::styled("  }\n", s),
            ])
        );
        assert_eq!(
            new_item(RG_JSON_END).to_spans(Some(replacement), None, PrintableStyle::Common),
            Spans::from("")
        );
        assert_eq!(
            new_item(RG_JSON_SUMMARY).to_spans(Some(replacement), None, PrintableStyle::Common),
            Spans::from("Search duration: 0.013911s")
        );
    }

    #[test]
    fn to_span_with_text_selected() {
        let s = Style::default();
        assert_eq!(
            new_item(RG_JSON_BEGIN).to_spans(None, Some(0), PrintableStyle::Common),
            Spans::from(vec![Span::styled(
                "src/model/item.rs",
                s.fg(Color::Black).bg(Color::Yellow)
            )])
        );
        assert_eq!(
            new_item(RG_JSON_MATCH).to_spans(None, Some(0), PrintableStyle::Common),
            Spans::from(vec![
                Span::styled("197:", s.fg(Color::Yellow)),
                Span::styled("    ", s.fg(Color::Yellow)),
                Span::styled("Item", s.fg(Color::Black).bg(Color::Yellow)),
                Span::styled("::new(", s.fg(Color::Yellow)),
                Span::styled("rg_msg", s.fg(Color::Black).bg(Color::Red)),
                Span::styled(")\n", s.fg(Color::Yellow)),
            ])
        );
        assert_eq!(
            new_item(RG_JSON_CONTEXT).to_spans(None, Some(0), PrintableStyle::Common),
            Spans::from(vec![
                Span::styled("198:", s.fg(Color::Yellow)),
                Span::styled("  }\n", s.fg(Color::Yellow)),
            ])
        );
        assert_eq!(
            new_item(RG_JSON_END).to_spans(None, Some(0), PrintableStyle::Common),
            Spans::from("")
        );
        assert_eq!(
            new_item(RG_JSON_SUMMARY).to_spans(None, Some(0), PrintableStyle::Common),
            Spans::from(vec![Span::styled(
                "Search duration: 0.013911s",
                s.fg(Color::Yellow)
            )])
        );
    }

    #[test]
    fn to_span_with_text_replacement_selected() {
        let s = Style::default();
        let replacement = "foobar";
        assert_eq!(
            new_item(RG_JSON_BEGIN).to_spans(Some(replacement), Some(0), PrintableStyle::Common),
            Spans::from(vec![Span::styled(
                "src/model/item.rs",
                s.fg(Color::Magenta)
            )])
        );
        assert_eq!(
            new_item(RG_JSON_MATCH).to_spans(Some(replacement), Some(0), PrintableStyle::Common),
            Spans::from(vec![
                Span::styled("197:", s),
                Span::styled("    ", s),
                Span::styled("Item", s.fg(Color::Red).add_modifier(Modifier::CROSSED_OUT)),
                Span::styled(replacement, s.fg(Color::Green)),
                Span::styled("::new(", s),
                Span::styled(
                    "rg_msg",
                    s.fg(Color::Red).add_modifier(Modifier::CROSSED_OUT)
                ),
                Span::styled(replacement, s.fg(Color::Green)),
                Span::styled(")\n", s),
            ])
        );
        assert_eq!(
            new_item(RG_JSON_CONTEXT).to_spans(Some(replacement), Some(0), PrintableStyle::Common),
            Spans::from(vec![Span::styled("198:", s), Span::styled("  }\n", s),])
        );
        assert_eq!(
            new_item(RG_JSON_END).to_spans(Some(replacement), Some(0), PrintableStyle::Common),
            Spans::from("")
        );
        assert_eq!(
            new_item(RG_JSON_SUMMARY).to_spans(Some(replacement), Some(0), PrintableStyle::Common),
            Spans::from("Search duration: 0.013911s")
        );
    }

    #[cfg(not(windows))] // FIXME: implement base64 tests for Windows
    #[test]
    fn to_span_with_base64_lossy() {
        // Since we don't read the entire file when we view the results, we expect the UTF8 replacement character.
        let s = Style::default();
        assert_eq!(
            new_item(RG_B64_JSON_BEGIN).to_spans(None, None, PrintableStyle::Common),
            Spans::from(vec![Span::styled("./a/fo�o", s.fg(Color::Magenta))])
        );
        assert_eq!(
            new_item(RG_B64_JSON_END).to_spans(None, None, PrintableStyle::Common),
            Spans::from("")
        );
        assert_eq!(
            new_item(RG_B64_JSON_MATCH).to_spans(None, None, PrintableStyle::Common),
            Spans::from(vec![
                Span::styled("197:", s.fg(Color::DarkGray)),
                Span::styled("    �", s),
                Span::styled("Item", s.bg(Color::Red).fg(Color::Black)),
                Span::styled("::�new(", s),
                Span::styled("rg_msg", s.bg(Color::Red).fg(Color::Black)),
                Span::styled(")\n", s),
            ])
        );
        assert_eq!(
            new_item(RG_B64_JSON_CONTEXT).to_spans(None, None, PrintableStyle::Common),
            Spans::from(vec![
                Span::styled("198:", s.fg(Color::DarkGray)),
                Span::styled("  �}\n", s)
            ])
        );
    }

    #[cfg(not(windows))] // FIXME: implement base64 tests for Windows
    #[test]
    fn to_span_with_base64_lossy_replacement() {
        let s = Style::default();
        let replacement = "foobar";
        assert_eq!(
            new_item(RG_B64_JSON_BEGIN).to_spans(Some(replacement), None, PrintableStyle::Common),
            Spans::from(vec![Span::styled("./a/fo�o", s.fg(Color::Magenta)),])
        );
        assert_eq!(
            new_item(RG_B64_JSON_END).to_spans(Some(replacement), None, PrintableStyle::Common),
            Spans::from("")
        );
        assert_eq!(
            new_item(RG_B64_JSON_MATCH).to_spans(Some(replacement), None, PrintableStyle::Common),
            Spans::from(vec![
                Span::styled("197:", s.fg(Color::DarkGray)),
                Span::styled("    �", s),
                Span::styled("Item", s.fg(Color::Red).add_modifier(Modifier::CROSSED_OUT)),
                Span::styled("foobar", s.fg(Color::Green)),
                Span::styled("::�new(", s),
                Span::styled(
                    "rg_msg",
                    s.fg(Color::Red).add_modifier(Modifier::CROSSED_OUT)
                ),
                Span::styled("foobar", s.fg(Color::Green)),
                Span::styled(")\n", s),
            ])
        );
        assert_eq!(
            new_item(RG_B64_JSON_CONTEXT).to_spans(Some(replacement), None, PrintableStyle::Common),
            Spans::from(vec![
                Span::styled("198:", s.fg(Color::DarkGray)),
                Span::styled("  �}\n", s)
            ])
        );
    }

    #[cfg(not(windows))] // FIXME: implement base64 tests for Windows
    #[test]
    fn to_span_with_base64_lossy_selected() {
        // Since we don't read the entire file when we view the results, we expect the UTF8 replacement character.
        let s = Style::default();
        assert_eq!(
            new_item(RG_B64_JSON_BEGIN).to_spans(None, Some(0), PrintableStyle::Common),
            Spans::from(vec![Span::styled(
                "./a/fo�o",
                s.fg(Color::Black).bg(Color::Yellow)
            )])
        );
        assert_eq!(
            new_item(RG_B64_JSON_END).to_spans(None, Some(0), PrintableStyle::Common),
            Spans::from("")
        );
        assert_eq!(
            new_item(RG_B64_JSON_MATCH).to_spans(None, Some(0), PrintableStyle::Common),
            Spans::from(vec![
                Span::styled("197:", s.fg(Color::Yellow)),
                Span::styled("    �", s.fg(Color::Yellow)),
                Span::styled("Item", s.fg(Color::Black).bg(Color::Yellow)),
                Span::styled("::�new(", s.fg(Color::Yellow)),
                Span::styled("rg_msg", s.bg(Color::Red).fg(Color::Black)),
                Span::styled(")\n", s.fg(Color::Yellow)),
            ])
        );
        assert_eq!(
            new_item(RG_B64_JSON_CONTEXT).to_spans(None, Some(0), PrintableStyle::Common),
            Spans::from(vec![
                Span::styled("198:", s.fg(Color::Yellow)),
                Span::styled("  �}\n", s.fg(Color::Yellow))
            ])
        );
    }

    #[cfg(not(windows))] // FIXME: implement base64 tests for Windows
    #[test]
    fn to_span_with_base64_lossy_replacement_selected() {
        // Since we don't read the entire file when we view the results, we expect the UTF8 replacement character.
        let s = Style::default();
        let replacement = "foobar";
        assert_eq!(
            new_item(RG_B64_JSON_BEGIN).to_spans(
                Some(replacement),
                Some(0),
                PrintableStyle::Common
            ),
            Spans::from(vec![Span::styled("./a/fo�o", s.fg(Color::Magenta))])
        );
        assert_eq!(
            new_item(RG_B64_JSON_END).to_spans(Some(replacement), Some(0), PrintableStyle::Common),
            Spans::from("")
        );
        assert_eq!(
            new_item(RG_B64_JSON_MATCH).to_spans(
                Some(replacement),
                Some(0),
                PrintableStyle::Common
            ),
            Spans::from(vec![
                Span::styled("197:", s),
                Span::styled("    �", s),
                Span::styled("Item", s.fg(Color::Red).add_modifier(Modifier::CROSSED_OUT)),
                Span::styled(replacement, s.fg(Color::Green)),
                Span::styled("::�new(", s),
                Span::styled(
                    "rg_msg",
                    s.fg(Color::Red).add_modifier(Modifier::CROSSED_OUT)
                ),
                Span::styled(replacement, s.fg(Color::Green)),
                Span::styled(")\n", s),
            ])
        );
        assert_eq!(
            new_item(RG_B64_JSON_CONTEXT).to_spans(
                Some(replacement),
                Some(0),
                PrintableStyle::Common
            ),
            Spans::from(vec![Span::styled("198:", s), Span::styled("  �}\n", s)])
        );
    }
}
