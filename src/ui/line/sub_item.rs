use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use crate::model::Printable;
use crate::rg::de::SubMatch;
use crate::ui::render::UiItemContext;

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
    /// A SubItem contains the "match". A match _may_ be over multiple lines, but there will only ever
    /// be a single span on each line. So this returns a list of "lines": one span for each line.
    pub fn to_span_lines(&self, ctx: &UiItemContext, is_item_selected: bool) -> Vec<Span> {
        let mut s = Style::default();
        if ctx.app_ui_state.is_replacing() {
            if self.should_replace {
                s = s.fg(Color::Red).add_modifier(Modifier::CROSSED_OUT);
            }
        } else if is_item_selected && ctx.app_list_state.selected_submatch() == self.index {
            if self.should_replace {
                s = s.fg(Color::Black).bg(Color::Yellow);
            } else {
                s = s.fg(Color::Yellow).bg(Color::DarkGray);
            }
        } else if self.should_replace {
            s = s.fg(Color::Black).bg(Color::Red);
        } else {
            s = s.fg(Color::Red).bg(Color::DarkGray);
        }

        self.sub_match
            .text
            .to_printable(ctx.printable_style)
            .lines()
            .map(|line| Span::styled(line.to_string(), s))
            .collect()
    }
}
