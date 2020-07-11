use std::path::PathBuf;

use tui::style::{Color, Modifier, Style, StyleDiff};
use tui::text::{Span, Spans};

use crate::rg::de::{ArbitraryData, RgMessage, RgMessageKind, SubMatch};

#[derive(Debug, Clone)]
pub struct SubItem {
    pub submatch: SubMatch,
    pub should_replace: bool,
}

impl SubItem {
    pub fn new(submatch: SubMatch) -> SubItem {
        SubItem {
            submatch,
            should_replace: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    pub kind: RgMessageKind,
    rg_message: RgMessage,

    sub_items: Vec<SubItem>,
}

impl Item {
    pub fn new(rg_message: RgMessage) -> Item {
        let kind = match &rg_message {
            RgMessage::Begin { .. } => RgMessageKind::Begin,
            RgMessage::End { .. } => RgMessageKind::End,
            RgMessage::Match { .. } => RgMessageKind::Match,
            RgMessage::Context { .. } => RgMessageKind::Context,
            RgMessage::Summary { .. } => RgMessageKind::Summary,
        };

        let sub_items = match &rg_message {
            RgMessage::Match { submatches, .. } => {
                submatches.iter().map(|s| SubItem::new(s.clone())).collect()
            }
            // TODO: ?
            _ => vec![],
        };

        Item {
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

    fn line_number_to_text<'a>(base_style: Style, selected: bool, line_number: usize) -> Span<'a> {
        Span::styled(
            format!("{}:", line_number),
            if selected {
                StyleDiff::from(base_style)
            } else {
                StyleDiff::from(base_style).fg(Color::DarkGray)
            },
        )
    }

    pub fn to_text(&self, replacement: Option<&str>, selected: Option<usize>) -> Spans {
        let mut base_style = Style::default();
        if selected.is_some() {
            base_style = base_style.fg(Color::Yellow);
        }

        // TODO: handle multiline matches
        match &self.rg_message {
            RgMessage::Begin { .. } => Spans::from(Span::styled(
                format!("{}", self.path_buf().unwrap().display()),
                if selected.is_some() {
                    StyleDiff::from(base_style)
                } else {
                    StyleDiff::from(base_style).fg(Color::Magenta)
                },
            )),
            RgMessage::Context {
                lines, line_number, ..
            } => {
                let mut spans = vec![];
                if let Some(n) = line_number {
                    spans.push(Item::line_number_to_text(
                        base_style,
                        selected.is_some(),
                        *n,
                    ));
                }

                spans.push(Span::styled(
                    lines.lossy_utf8(),
                    StyleDiff::from(base_style),
                ));
                Spans::from(spans)
            }
            RgMessage::Match {
                lines, line_number, ..
            } => {
                let mut spans = vec![];
                if let Some(n) = line_number {
                    spans.push(Item::line_number_to_text(
                        base_style,
                        selected.is_some(),
                        *n,
                    ));
                }

                let lines_text = lines.lossy_utf8();

                let mut start = 0;
                for (
                    idx,
                    SubItem {
                        should_replace,
                        submatch,
                    },
                ) in self.sub_items.iter().enumerate()
                {
                    // Text inbetween start (or last submatch) and this submatch.
                    let before_submatch = start..submatch.range.start;
                    start = submatch.range.end;

                    #[allow(clippy::len_zero)]
                    if before_submatch.len() > 0 {
                        spans.push(Span::styled(
                            lines_text[before_submatch].to_string(),
                            StyleDiff::from(base_style),
                        ));
                    }

                    // TODO: simplify and clean up
                    let mut submatch_style = selected
                        .map(|i| {
                            let mut s = base_style;
                            if i == idx && replacement.is_none() {
                                s = s.bg(Color::Yellow);
                            }

                            if *should_replace {
                                s.fg(Color::Red)
                            } else {
                                s.fg(Color::DarkGray)
                            }
                        })
                        .or_else(|| {
                            if *should_replace {
                                Some(base_style.fg(Color::Red))
                            } else {
                                Some(base_style.fg(Color::DarkGray))
                            }
                        })
                        .unwrap();

                    if *should_replace && replacement.is_some() {
                        submatch_style = submatch_style.modifier(Modifier::CROSSED_OUT);
                    }

                    // This submatch.
                    spans.push(Span::styled(
                        lines_text[submatch.range.clone()].to_string(),
                        StyleDiff::from(submatch_style),
                    ));

                    // Replacement text.
                    if *should_replace {
                        if let Some(replacement) = replacement {
                            spans.push(Span::styled(
                                replacement.to_owned(),
                                StyleDiff::from(base_style).fg(Color::Green),
                            ));
                        }
                    }
                }

                // Text after the last submatch and before the end of the line.
                let trailing = start..lines_text.len();
                #[allow(clippy::len_zero)]
                if trailing.len() > 0 {
                    spans.push(Span::styled(
                        lines_text[trailing].to_string(),
                        StyleDiff::from(base_style),
                    ));
                }

                Spans::from(spans)
            }
            RgMessage::End { .. } => Spans::from(""),
            RgMessage::Summary { elapsed_total, .. } => Spans::from(Span::styled(
                format!("Search duration: {}", elapsed_total.human),
                StyleDiff::from(base_style),
            )),
        }
    }
}

#[cfg(test_ignored)]
mod tests {
    use std::path::PathBuf;

    use pretty_assertions::assert_eq;
    use tui::style::{Color, Style};
    use tui::widgets::Text;

    use crate::model::*;
    use crate::rg::de::test_utilities::*;
    use crate::rg::de::*;

    const RG_JSON_BEGIN: &str = r#"{"type":"begin","data":{"path":{"text":"src/model/item.rs"}}}"#;
    const RG_JSON_MATCH: &str = r#"{"type":"match","data":{"path":{"text":"src/model/item.rs"},"lines":{"text":"    Item::new(rg_msg)\n"},"line_number":197,"absolute_offset":5522,"submatches":[{"match":{"text":"rg_msg"},"start":14,"end":20}]}}"#;
    const RG_JSON_CONTEXT: &str = r#"{"type":"context","data":{"path":{"text":"src/model/item.rs"},"lines":{"text":"  }\n"},"line_number":198,"absolute_offset":5544,"submatches":[]}}"#;
    const RG_JSON_END: &str = r#"{"type":"end","data":{"path":{"text":"src/model/item.rs"},"binary_offset":null,"stats":{"elapsed":{"secs":0,"nanos":97924,"human":"0.000098s"},"searches":1,"searches_with_match":1,"bytes_searched":5956,"bytes_printed":674,"matched_lines":2,"matches":2}}}"#;
    const RG_JSON_SUMMARY: &str = r#"{"data":{"elapsed_total":{"human":"0.013911s","nanos":13911027,"secs":0},"stats":{"bytes_printed":3248,"bytes_searched":18789,"elapsed":{"human":"0.000260s","nanos":260276,"secs":0},"matched_lines":10,"matches":10,"searches":2,"searches_with_match":2}},"type":"summary"}"#;

    fn new_item(raw_json: &str) -> Item {
        let rg_msg = serde_json::from_str::<RgMessage>(raw_json).unwrap();
        Item::new(rg_msg)
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
        assert_eq!(new_item(RG_JSON_BEGIN).match_count(), 0);
        assert_eq!(new_item(RG_JSON_MATCH).match_count(), 1);
        assert_eq!(new_item(RG_JSON_CONTEXT).match_count(), 0);
        assert_eq!(new_item(RG_JSON_END).match_count(), 0);
        assert_eq!(new_item(RG_JSON_SUMMARY).match_count(), 0);
    }

    #[test]
    fn matches() {
        assert_eq!(new_item(RG_JSON_BEGIN).matches(), None);
        assert_eq!(
            new_item(RG_JSON_MATCH).matches(),
            Some([SubMatch::new_text("rg_msg", 14..20)].as_ref())
        );
        assert_eq!(new_item(RG_JSON_CONTEXT).matches(), None);
        assert_eq!(new_item(RG_JSON_END).matches(), None);
        assert_eq!(new_item(RG_JSON_SUMMARY).matches(), None);
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
    fn to_text_with_text() {
        let s = Style::default();

        // Without replacement.
        assert_eq!(
            new_item(RG_JSON_BEGIN).to_text(None),
            Text::styled("src/model/item.rs", s.fg(Color::Magenta))
        );
        assert_eq!(
            new_item(RG_JSON_MATCH).to_text(None),
            Text::styled("197:    Item::new(rg_msg)\n", s)
        );
        assert_eq!(
            new_item(RG_JSON_CONTEXT).to_text(None),
            Text::styled("198:  }\n", s.fg(Color::DarkGray))
        );
        assert_eq!(new_item(RG_JSON_END).to_text(None), Text::raw(""));
        assert_eq!(
            new_item(RG_JSON_SUMMARY).to_text(None),
            Text::raw("Search duration: 0.013911s")
        );

        // With replacement.
        let replacement = "foobar";
        assert_eq!(
            new_item(RG_JSON_BEGIN).to_text(Some(replacement)),
            Text::styled("src/model/item.rs", s.fg(Color::Magenta))
        );
        assert_eq!(
            new_item(RG_JSON_MATCH).to_text(Some(replacement)),
            Text::styled("197:    Item::new(foobar)\n", s)
        );
        assert_eq!(
            new_item(RG_JSON_CONTEXT).to_text(Some(replacement)),
            Text::styled("198:  }\n", s.fg(Color::DarkGray))
        );
        assert_eq!(
            new_item(RG_JSON_END).to_text(Some(replacement)),
            Text::raw("")
        );
        assert_eq!(
            new_item(RG_JSON_SUMMARY).to_text(Some(replacement)),
            Text::raw("Search duration: 0.013911s")
        );
    }

    #[test]
    fn to_text_with_base64_lossy() {
        // The following types are skipped because:
        // Begin:   already tested via the `path_with_base64` test.
        // End:     already tested via the `path_with_base64` test.
        // Summary: doesn't include an `ArbitraryData` struct.

        let b64_json_match = r#"{"type":"match","data":{"path":{"text":"src/model/item.rs"},"lines":{"bytes":"ICAgIEl0ZW06Ov9uZXcocmdfbXNnKQo="},"line_number":197,"absolute_offset":5522,"submatches":[{"match":{"text":"rg_msg"},"start":15,"end":21}]}}"#;
        let b64_json_context = r#"{"type":"context","data":{"path":{"text":"src/model/item.rs"},"lines":{"bytes":"ICD/fQo="},"line_number":198,"absolute_offset":5544,"submatches":[]}}"#;

        // Since we don't read the entire file when we view the results, we expect the UTF8 replacement character.
        // Without replacement.
        let s = Style::default();
        assert_eq!(
            new_item(b64_json_match).to_text(None),
            Text::styled("197:    Item::�new(rg_msg)\n", s)
        );
        assert_eq!(
            new_item(b64_json_context).to_text(None),
            Text::styled("198:  �}\n", s.fg(Color::DarkGray))
        );

        // With replacement.
        let replacement = "foobar";
        assert_eq!(
            new_item(b64_json_match).to_text(Some(replacement)),
            Text::styled("197:    Item::�new(foobar)\n", s)
        );
        assert_eq!(
            new_item(b64_json_context).to_text(Some(replacement)),
            Text::styled("198:  �}\n", s.fg(Color::DarkGray))
        );
    }
}
