use std::ops::Range;
use std::path::PathBuf;

use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use unicode_width::UnicodeWidthStr;

use crate::model::{Printable, PrintableStyle};
use crate::rg::de::{ArbitraryData, RgMessage, RgMessageKind, SubMatch};
use crate::ui::app::AppUiState;
use crate::ui::render::UiItemContext;

macro_rules! format_line_number {
    ($content:expr) => {
        format!("{}:", $content)
    };
}

fn line_count(available_width: usize, text: impl AsRef<str>) -> usize {
    #[cfg(not(release))]
    assert!(available_width != 0);

    let line_width = text.as_ref().width();
    // lines that wrap
    let mut count = line_width / available_width;
    // any remainder on the last line
    if line_width % available_width > 0 {
        count += 1;
    }

    count
}

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
    pub fn line_count(&self, list_width: u16, style: PrintableStyle) -> usize {
        let list_width = list_width as usize;
        self.sub_match
            .text
            .to_printable(style)
            .lines()
            .map(|line| line_count(list_width, line))
            .sum::<usize>()
    }

    /// A SubItem contains the "match". A match _may_ be over multiple lines, but there will only ever
    /// be a single span on each line. So this returns a list of "lines": one span for each line.
    pub fn to_span_lines(&self, ctx: &UiItemContext, is_item_selected: bool) -> Vec<Span> {
        let mut s = Style::default();
        if ctx.app_ui_state.is_replacing() {
            if self.should_replace {
                s = s.fg(Color::Red).add_modifier(Modifier::CROSSED_OUT);
            }
        } else {
            if is_item_selected && ctx.app_list_state.selected_submatch() == self.index {
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

    pub fn line_count(&self, list_width: u16, style: PrintableStyle) -> usize {
        match &self.rg_message {
            RgMessage::Begin { .. } | RgMessage::End { .. } => 1,
            RgMessage::Match { lines, .. } | RgMessage::Context { lines, .. } => {
                let list_width = list_width as usize;
                let line_number = self.line_number().unwrap();
                lines
                    .to_printable(style)
                    .lines()
                    .enumerate()
                    .map(|(i, line)| {
                        let line_number = format_line_number!(line_number + i);
                        let available_width = list_width.saturating_sub(line_number.width());
                        line_count(available_width, line)
                    })
                    .sum::<usize>()
            }
            RgMessage::Summary { .. } => 0,
        }
    }

    pub fn to_span_lines(&self, ctx: &UiItemContext) -> Vec<Spans> {
        let is_replacing = ctx.app_ui_state.is_replacing();
        let is_selected = ctx.app_list_state.selected_item() == self.index;

        let mut base_style = Style::default();
        if !is_replacing && is_selected {
            base_style = base_style.fg(Color::Yellow);
        }

        // pushes a span to `spans` which contains the given line number content
        macro_rules! push_line_number_span {
            ($spans:expr, $content:expr) => {{
                let mut line_number_style = base_style;
                if !is_selected || is_replacing {
                    line_number_style = line_number_style.fg(Color::DarkGray);
                }

                $spans.push(Span::styled(
                    format_line_number!($content),
                    line_number_style,
                ));
            }};
        };

        let span_lines = match &self.rg_message {
            RgMessage::Begin { .. } => vec![vec![Span::styled(
                format!("{}", self.path_buf().unwrap().display()).to_printable(ctx.printable_style),
                if !is_replacing && is_selected {
                    base_style.fg(Color::Black).bg(Color::Yellow)
                } else {
                    base_style.fg(Color::Magenta)
                },
            )]],

            RgMessage::Context {
                lines, line_number, ..
            } => {
                let mut span_lines = vec![];
                for (i, line) in lines.to_printable(ctx.printable_style).lines().enumerate() {
                    let mut spans = vec![];
                    if i == 0 {
                        if let Some(n) = line_number {
                            push_line_number_span!(spans, n);
                        }
                    }

                    spans.push(Span::styled(line.to_string(), base_style));
                    span_lines.push(spans);
                }

                span_lines
            }

            RgMessage::Match {
                lines, line_number, ..
            } => {
                let mut line_number = line_number.clone();

                // Read the lines as bytes since we split it at the ranges that ripgrep gives us in each of the submatches.
                let lines_bytes = lines.to_vec();
                let replacement_spans = ctx.replacement_text.map(|text| {
                    let replacement_style = base_style.fg(Color::Green);
                    let mut spans = text
                        .to_printable(ctx.printable_style)
                        .lines()
                        .map(|line| Span::styled(line.to_owned(), replacement_style))
                        .collect::<Vec<_>>();

                    // NOTE: since `"foo\n".lines().collect()` == `vec!["foo"]` we need to make sure the
                    // last newline isn't trimmed.
                    if !ctx.printable_style.is_one_line() && text.ends_with("\n") {
                        spans.push(Span::from(""));
                    }

                    spans
                });

                let mut span_lines = vec![];
                let mut spans = vec![]; // filled and emptied for each line

                macro_rules! push_utf8_slice {
                    ($range:ident) => {
                        {
                            let mut content = String::from_utf8_lossy(&lines_bytes[$range]).to_printable(ctx.printable_style);
                            // remove trailing new line if one exists since lines are already handled
                            if content.ends_with("\n") {
                                content.pop();
                            }
                            // NOTE: don't handle multiple lines in the match because AFAICT ripgrep doesn't return multiline
                            // text in between submatches in a "match" item.
                            spans.push(Span::styled(content, base_style));
                        }
                    }
                }

                // Don't create a new Spans for the last line in the lines returned from the submatches or the replacement
                // text, since there may be text appended afterwards to the lines later on (in the case of submatches, the
                // replacement text, and for replacement text any remaining non-match text from the line).
                macro_rules! new_line_if_needed {
                    ($len:expr, $idx:expr) => {
                        if $idx != $len - 1 {
                            span_lines.push(spans.drain(..).collect::<Vec<Span>>());
                        }
                    };
                }

                let mut offset = 0;
                for (idx, sub_item) in self.sub_items.iter().enumerate() {
                    let Range { start, end } = sub_item.sub_match.range;

                    if idx == 0 {
                        if let Some(n) = line_number {
                            push_line_number_span!(spans, n);
                        }
                    }

                    // Text in between start (or last SubMatch) and this SubMatch.
                    let leading = offset..start;
                    #[allow(clippy::len_zero)]
                    if leading.len() > 0 {
                        push_utf8_slice!(leading);
                    }

                    // Match text, also may contain any leading line numbers and text from before.
                    let confirm_replacement =
                        matches!(ctx.app_ui_state, AppUiState::ConfirmReplacement(_));
                    if !confirm_replacement || !sub_item.should_replace {
                        let sub_span_lines = sub_item.to_span_lines(ctx, is_selected);
                        let sub_span_lines_len = sub_span_lines.len();
                        for (i, span) in sub_span_lines.into_iter().enumerate() {
                            if i > 0 {
                                if is_replacing {
                                    push_line_number_span!(spans, "-");
                                } else if let Some(n) = line_number.as_mut() {
                                    *n += 1;
                                    push_line_number_span!(spans, n);
                                }
                            }

                            spans.push(span);
                            new_line_if_needed!(sub_span_lines_len, i);
                        }
                    }

                    // Replacement text.
                    if sub_item.should_replace {
                        if let Some(replacement_span_lines) = replacement_spans.as_ref() {
                            for (i, span) in replacement_span_lines.iter().enumerate() {
                                if i == 0 {
                                    // reset the line number
                                    line_number = self.line_number().cloned();
                                } else {
                                    push_line_number_span!(spans, "+");
                                }

                                spans.push(span.clone());
                                new_line_if_needed!(replacement_span_lines.len(), i);
                            }
                        }
                    }

                    offset = end;
                }

                // Text after the last SubMatch and before the end of the line.
                let trailing = offset..lines_bytes.len();
                #[allow(clippy::len_zero)]
                if trailing.len() > 0 {
                    push_utf8_slice!(trailing);
                }

                span_lines.push(spans);
                span_lines
            }
            RgMessage::End { .. } => vec![vec![Span::from("")]],
            // NOTE: the summary item is not added to the app's list of items
            RgMessage::Summary { .. } => unreachable!(),
        };

        // wrap lines
        let max_width = ctx.list_rect.width as usize;
        span_lines
            .into_iter()
            .flat_map(|spans| {
                use unicode_width::UnicodeWidthChar;

                let mut wrapped_spans = vec![];
                let mut tmp = vec![];
                let mut len = 0;
                for span in spans {
                    let span_width = span.width();
                    if len + span_width > max_width {
                        let mut chars = vec![];
                        for ch in span.content.chars() {
                            // NOTE: all control characters (except "\n") should have been removed via the `Printable` trait
                            // and "\n" should have been removed when building Spans from the item
                            let char_width = ch.width().expect(
                                "encountered unexpected control character while wrapping lines",
                            );
                            if len + char_width > max_width {
                                tmp.push(Span::styled(
                                    chars.drain(..).collect::<String>(),
                                    span.style,
                                ));
                                wrapped_spans.push(Spans::from(tmp.drain(..).collect::<Vec<_>>()));
                                len = 0;
                            }

                            len += char_width;
                            chars.push(ch);
                        }

                        let remaining_span =
                            Span::styled(chars.drain(..).collect::<String>(), span.style);
                        tmp.push(remaining_span);
                    } else {
                        len += span_width;
                        tmp.push(span);
                    }
                }

                wrapped_spans.push(Spans::from(tmp.drain(..).collect::<Vec<_>>()));
                wrapped_spans
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use insta::assert_debug_snapshot;
    use pretty_assertions::assert_eq;
    use tui::layout::Rect;

    use crate::model::*;
    use crate::rg::de::test_utilities::*;
    use crate::rg::de::*;
    use crate::ui::app::{AppListState, AppUiState};
    use crate::ui::render::UiItemContext;

    pub fn new_item(raw_json: &str) -> Item {
        Item::new(0, RgMessage::from_str(raw_json))
    }

    #[test]
    fn item_kind_matches_rg_message_kind() {
        assert_eq!(new_item(RG_JSON_BEGIN).kind, RgMessageKind::Begin);
        assert_eq!(new_item(RG_JSON_MATCH_MULTILINE).kind, RgMessageKind::Match);
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
                SubItem::new(0, SubMatch::new_text("Item", 4..8)),
                SubItem::new(1, SubMatch::new_text("rg_msg", 14..20))
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
                0,
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

    fn new_ui_item_ctx<'a>(
        replacement_text: Option<&'a str>,
        app_list_state: &'a AppListState,
        app_ui_state: &'a AppUiState,
    ) -> UiItemContext<'a> {
        match &replacement_text {
            Some(_) => assert!(app_ui_state.is_replacing()),
            None => assert!(!app_ui_state.is_replacing()),
        }

        UiItemContext {
            printable_style: PrintableStyle::Hidden,
            replacement_text,
            app_list_state,
            app_ui_state,
            list_rect: Rect::new(0, 0, 80, 24),
        }
    }

    fn new_app_list_state() -> AppListState {
        let mut list_state = AppListState::new();
        list_state.set_indicator_pos(999);
        list_state.set_selected_item(999);
        list_state.set_selected_submatch(999);
        list_state
    }

    #[test]
    fn to_span_lines_with_text() {
        let app_list_state = new_app_list_state();
        let app_ui_state = AppUiState::SelectMatches;
        let ctx = new_ui_item_ctx(None, &app_list_state, &app_ui_state);

        assert_debug_snapshot!(new_item(RG_JSON_BEGIN).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_JSON_MATCH).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_JSON_CONTEXT).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_JSON_END).to_span_lines(&ctx));
    }

    #[test]
    fn to_span_lines_with_text_input_replacement() {
        let replacement = "foobar";
        let app_list_state = new_app_list_state();
        let app_ui_state = AppUiState::InputReplacement(String::from(replacement));
        let ctx = new_ui_item_ctx(Some(replacement), &app_list_state, &app_ui_state);

        assert_debug_snapshot!(new_item(RG_JSON_BEGIN).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_JSON_MATCH).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_JSON_CONTEXT).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_JSON_END).to_span_lines(&ctx));
    }

    #[test]
    fn to_span_lines_with_text_confirm_replacement() {
        let replacement = "foobar";
        let app_list_state = new_app_list_state();
        let app_ui_state = AppUiState::ConfirmReplacement(String::from(replacement));
        let ctx = new_ui_item_ctx(Some(replacement), &app_list_state, &app_ui_state);

        assert_debug_snapshot!(new_item(RG_JSON_BEGIN).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_JSON_MATCH).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_JSON_CONTEXT).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_JSON_END).to_span_lines(&ctx));
    }

    #[test]
    fn to_span_lines_with_text_selected() {
        let mut app_list_state = new_app_list_state();
        app_list_state.set_selected_item(0);
        app_list_state.set_selected_submatch(0);
        let app_ui_state = AppUiState::SelectMatches;
        let ctx = new_ui_item_ctx(None, &app_list_state, &app_ui_state);

        assert_debug_snapshot!(new_item(RG_JSON_BEGIN).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_JSON_MATCH).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_JSON_CONTEXT).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_JSON_END).to_span_lines(&ctx));
    }

    #[test]
    fn to_span_lines_with_deselected_submatch() {
        let mut app_list_state = new_app_list_state();
        app_list_state.set_selected_item(0);
        app_list_state.set_selected_submatch(0);
        let app_ui_state = AppUiState::SelectMatches;
        let ctx = new_ui_item_ctx(None, &app_list_state, &app_ui_state);

        let mut item = new_item(RG_JSON_MATCH);
        item.set_should_replace(0, false);

        assert_debug_snapshot!(item.to_span_lines(&ctx));
    }

    #[test]
    fn to_span_lines_with_deselected_submatch_input_replacement() {
        let mut app_list_state = new_app_list_state();
        app_list_state.set_selected_item(0);
        app_list_state.set_selected_submatch(0);
        let replacement = "foobar";
        let app_ui_state = AppUiState::InputReplacement(String::from(replacement));
        let ctx = new_ui_item_ctx(Some(replacement), &app_list_state, &app_ui_state);

        let mut item = new_item(RG_JSON_MATCH);
        item.set_should_replace(0, false);

        assert_debug_snapshot!(item.to_span_lines(&ctx));
    }

    #[test]
    fn to_span_lines_with_deselected_submatch_confirm_replacement() {
        let mut app_list_state = new_app_list_state();
        app_list_state.set_selected_item(0);
        app_list_state.set_selected_submatch(0);
        let replacement = "foobar";
        let app_ui_state = AppUiState::ConfirmReplacement(String::from(replacement));
        let ctx = new_ui_item_ctx(Some(replacement), &app_list_state, &app_ui_state);

        let mut item = new_item(RG_JSON_MATCH);
        item.set_should_replace(0, false);

        assert_debug_snapshot!(item.to_span_lines(&ctx));
    }

    #[cfg(not(windows))] // FIXME: implement base64 tests for Windows
    #[test]
    fn to_span_lines_with_base64_lossy() {
        // Since we don't read the entire file when we view the results, we expect the UTF8 replacement character.
        let app_list_state = new_app_list_state();
        let app_ui_state = AppUiState::SelectMatches;
        let ctx = new_ui_item_ctx(None, &app_list_state, &app_ui_state);

        assert_debug_snapshot!(new_item(RG_B64_JSON_BEGIN).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_B64_JSON_END).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_B64_JSON_MATCH).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_B64_JSON_CONTEXT).to_span_lines(&ctx));
    }

    #[cfg(not(windows))] // FIXME: implement base64 tests for Windows
    #[test]
    fn to_span_lines_with_base64_lossy_input_replacement() {
        let replacement = "foobar";
        let app_list_state = new_app_list_state();
        let app_ui_state = AppUiState::InputReplacement(String::from(replacement));
        let ctx = new_ui_item_ctx(Some(replacement), &app_list_state, &app_ui_state);

        assert_debug_snapshot!(new_item(RG_B64_JSON_BEGIN).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_B64_JSON_END).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_B64_JSON_MATCH).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_B64_JSON_CONTEXT).to_span_lines(&ctx));
    }

    #[cfg(not(windows))] // FIXME: implement base64 tests for Windows
    #[test]
    fn to_span_lines_with_base64_lossy_confirm_replacement() {
        let replacement = "foobar";
        let app_list_state = new_app_list_state();
        let app_ui_state = AppUiState::ConfirmReplacement(String::from(replacement));
        let ctx = new_ui_item_ctx(Some(replacement), &app_list_state, &app_ui_state);

        assert_debug_snapshot!(new_item(RG_B64_JSON_BEGIN).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_B64_JSON_END).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_B64_JSON_MATCH).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_B64_JSON_CONTEXT).to_span_lines(&ctx));
    }

    #[cfg(not(windows))] // FIXME: implement base64 tests for Windows
    #[test]
    fn to_span_lines_with_base64_lossy_selected() {
        // Since we don't read the entire file when we view the results, we expect the UTF8 replacement character.
        let mut app_list_state = new_app_list_state();
        app_list_state.set_selected_item(0);
        app_list_state.set_selected_submatch(0);
        let app_ui_state = AppUiState::SelectMatches;
        let ctx = new_ui_item_ctx(None, &app_list_state, &app_ui_state);

        assert_debug_snapshot!(new_item(RG_B64_JSON_BEGIN).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_B64_JSON_END).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_B64_JSON_MATCH).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_B64_JSON_CONTEXT).to_span_lines(&ctx));
    }

    #[test]
    fn to_span_lines_with_multiline_replacement() {
        let replacement = "foobar\nbaz\nasdf";
        let app_list_state = new_app_list_state();
        let app_ui_state = AppUiState::InputReplacement(String::from(replacement));
        let ctx = new_ui_item_ctx(Some(replacement), &app_list_state, &app_ui_state);

        assert_debug_snapshot!(new_item(RG_JSON_MATCH).to_span_lines(&ctx));
    }

    #[test]
    fn to_span_lines_with_multiline_matches() {
        let app_list_state = new_app_list_state();
        let app_ui_state = AppUiState::SelectMatches;
        let ctx = new_ui_item_ctx(None, &app_list_state, &app_ui_state);

        assert_debug_snapshot!(new_item(RG_JSON_MATCH_MULTILINE).to_span_lines(&ctx));
    }

    #[test]
    fn to_span_lines_multiline_input_replacement_with_multiline_matches() {
        let replacement = "foobar\nbaz\nasdf";
        let app_list_state = new_app_list_state();
        let app_ui_state = AppUiState::InputReplacement(String::from(replacement));
        let ctx = new_ui_item_ctx(Some(replacement), &app_list_state, &app_ui_state);

        assert_debug_snapshot!(new_item(RG_JSON_MATCH_MULTILINE).to_span_lines(&ctx));
    }

    #[test]
    fn to_span_lines_multiline_confirm_replacement_with_multiline_matches() {
        let replacement = "foobar\nbaz\nasdf";
        let app_list_state = new_app_list_state();
        let app_ui_state = AppUiState::ConfirmReplacement(String::from(replacement));
        let ctx = new_ui_item_ctx(Some(replacement), &app_list_state, &app_ui_state);

        assert_debug_snapshot!(new_item(RG_JSON_MATCH_MULTILINE).to_span_lines(&ctx));
    }

    #[test]
    fn to_span_lines_line_wrapping() {
        let mut app_list_state = new_app_list_state();
        app_list_state.set_selected_item(0);
        app_list_state.set_selected_submatch(0);
        let app_ui_state = AppUiState::SelectMatches;
        let ctx = new_ui_item_ctx(None, &app_list_state, &app_ui_state);

        assert_debug_snapshot!(new_item(RG_JSON_MATCH_LINE_WRAP).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_JSON_CONTEXT_LINE_WRAP).to_span_lines(&ctx));
    }

    #[test]
    fn to_span_lines_line_wrapping_input_replacement() {
        let replacement = "foobar";
        let mut app_list_state = new_app_list_state();
        app_list_state.set_selected_item(0);
        app_list_state.set_selected_submatch(0);
        let app_ui_state = AppUiState::InputReplacement(String::from(replacement));
        let ctx = new_ui_item_ctx(Some(replacement), &app_list_state, &app_ui_state);

        assert_debug_snapshot!(new_item(RG_JSON_MATCH_LINE_WRAP).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_JSON_CONTEXT_LINE_WRAP).to_span_lines(&ctx));
    }

    #[test]
    fn to_span_lines_line_wrapping_confirm_replacement() {
        let replacement = "foobar";
        let mut app_list_state = new_app_list_state();
        app_list_state.set_selected_item(0);
        app_list_state.set_selected_submatch(0);
        let app_ui_state = AppUiState::ConfirmReplacement(String::from(replacement));
        let ctx = new_ui_item_ctx(Some(replacement), &app_list_state, &app_ui_state);

        assert_debug_snapshot!(new_item(RG_JSON_MATCH_LINE_WRAP).to_span_lines(&ctx));
        assert_debug_snapshot!(new_item(RG_JSON_CONTEXT_LINE_WRAP).to_span_lines(&ctx));
    }

    #[test]
    fn line_count_hidden() {
        let w = 80_u16;
        let s = PrintableStyle::Hidden;
        assert_eq!(new_item(RG_JSON_BEGIN).line_count(w, s), 1);
        assert_eq!(new_item(RG_JSON_MATCH).line_count(w, s), 1);
        assert_eq!(new_item(RG_JSON_MATCH_LINE_WRAP).line_count(w, s), 2);
        assert_eq!(new_item(RG_JSON_MATCH_LINE_WRAP_MULTI).line_count(w, s), 3);
        assert_eq!(new_item(RG_JSON_CONTEXT_LINE_WRAP).line_count(w, s), 2);
        assert_eq!(new_item(RG_JSON_CONTEXT).line_count(w, s), 1);
        assert_eq!(new_item(RG_JSON_END).line_count(w, s), 1);
        assert_eq!(new_item(RG_JSON_SUMMARY).line_count(w, s), 0);
    }

    macro_rules! assert_line_count {
        ($json:expr, $width:expr, $style:expr, $line_count:expr, $submatch_counts:expr) => {{
            let item = new_item($json);
            let line_count = item.line_count($width, $style);

            let expected_submatch_counts: &[usize] = $submatch_counts;
            let actual_submatch_counts: Vec<usize> = item
                .sub_items
                .iter()
                .map(|s| s.line_count($width, $style))
                .collect();
            assert_eq!(
                (line_count, &actual_submatch_counts[..]),
                ($line_count, expected_submatch_counts)
            );
        }};
    }

    #[test]
    fn line_count_multiple_lines() {
        let w = 80_u16;
        let styles = vec![
            PrintableStyle::Hidden,
            PrintableStyle::All(false),
            PrintableStyle::Common(false),
        ];
        for s in styles {
            assert_line_count!(RG_JSON_BEGIN, w, s, 1, &[]);
            assert_line_count!(RG_JSON_MATCH, w, s, 1, &[1, 1]);
            assert_line_count!(RG_JSON_MATCH_MULTILINE, w, s, 3, &[3, 1]);
            assert_line_count!(RG_JSON_MATCH_LINE_WRAP, w, s, 2, &[1]);
            assert_line_count!(
                RG_JSON_MATCH_LINE_WRAP_MULTI,
                w,
                s,
                3,
                &[1, 1, 1, 1, 1, 1, 1]
            );
            assert_line_count!(RG_JSON_CONTEXT_LINE_WRAP, w, s, 2, &[]);
            assert_line_count!(RG_JSON_CONTEXT, w, s, 1, &[]);
            assert_line_count!(RG_JSON_END, w, s, 1, &[]);
            assert_line_count!(RG_JSON_SUMMARY, w, s, 0, &[]);
        }
    }
    #[test]
    fn line_count_one_line() {
        let w = 80_u16;
        let styles = vec![PrintableStyle::All(true), PrintableStyle::Common(true)];
        for s in styles {
            assert_line_count!(RG_JSON_BEGIN, w, s, 1, &[]);
            assert_line_count!(RG_JSON_MATCH, w, s, 1, &[1, 1]);
            assert_line_count!(RG_JSON_MATCH_MULTILINE, w, s, 1, &[1, 1]);
            assert_line_count!(RG_JSON_MATCH_LINE_WRAP, w, s, 2, &[1]);
            assert_line_count!(
                RG_JSON_MATCH_LINE_WRAP_MULTI,
                w,
                s,
                3,
                &[1, 1, 1, 1, 1, 1, 1]
            );
            assert_line_count!(RG_JSON_CONTEXT_LINE_WRAP, w, s, 2, &[]);
            assert_line_count!(RG_JSON_CONTEXT, w, s, 1, &[]);
            assert_line_count!(RG_JSON_END, w, s, 1, &[]);
            assert_line_count!(RG_JSON_SUMMARY, w, s, 0, &[]);
        }
    }

    #[test]
    fn line_numbers_with_line_wrap_multi_submatch_input_replacement_multiline() {
        let replacement = "zip\nzap";
        let mut app_list_state = new_app_list_state();
        app_list_state.set_selected_item(0);
        app_list_state.set_selected_submatch(0);
        let app_ui_state = AppUiState::InputReplacement(String::from(replacement));
        let ctx = new_ui_item_ctx(Some(replacement), &app_list_state, &app_ui_state);

        assert_debug_snapshot!(new_item(RG_JSON_MATCH_LINE_WRAP_MULTI).to_span_lines(&ctx));
    }

    #[test]
    fn to_span_lines_input_replacement_trailing_line_feed() {
        let replacement = "foobar\n";
        let app_list_state = new_app_list_state();
        let app_ui_state = AppUiState::InputReplacement(String::from(replacement));
        let mut ctx = new_ui_item_ctx(Some(replacement), &app_list_state, &app_ui_state);

        // Should add a line
        assert_debug_snapshot!(new_item(RG_JSON_MATCH).to_span_lines(&ctx));

        // Should not add a line
        ctx.printable_style = ctx.printable_style.as_one_line();
        assert_debug_snapshot!(new_item(RG_JSON_MATCH).to_span_lines(&ctx));
    }
}
