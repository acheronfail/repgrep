/// Rendering for `App`.
use clap::crate_name;
use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table, Wrap};
use tui::Frame;

use crate::model::Printable;
use crate::rg::de::RgMessageKind;
use crate::ui::app::{App, AppUiState};
use crate::ui::render::UiItemContext;

const LIST_HIGHLIGHT_SYMBOL: &str = "-> ";

impl App {
    // The UI is:
    // _
    // | - list
    // | - of
    // | - matches
    // | status line (rg command line, matches, replacements, etc)
    // | command line (user input for replacement text, etc)
    // _
    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
        let (root_split, stats_and_input_split) = self.get_layouts(f.size());
        if matches!(self.ui_state, AppUiState::Help) {
            self.draw_help_view(f, root_split[0]);
        } else {
            self.draw_main_view(f, root_split[0]);
        }
        self.draw_stats_line(f, stats_and_input_split[0]);
        self.draw_input_line(f, stats_and_input_split[1]);
    }

    fn get_layouts(&self, r: Rect) -> (Vec<Rect>, Vec<Rect>) {
        let root_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)].as_ref())
            .split(r);

        let stats_and_input_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)].as_ref())
            .split(root_split[1]);

        (root_split, stats_and_input_split)
    }

    fn draw_input_line<B: Backend>(&mut self, f: &mut Frame<B>, r: Rect) {
        let prefix = "Replacement: ";
        let mut spans = match &self.ui_state {
            AppUiState::Help => vec![Span::from("Viewing Help. Press <esc> or <q> to return...")],
            AppUiState::SelectMatches => vec![Span::from(
                "Select (or deselect) Matches with <space> then press <Enter>. Press <?> for help.",
            )],
            AppUiState::InputReplacement(input) => vec![
                Span::from(prefix),
                if input.is_empty() {
                    Span::styled("<empty>", Style::default().fg(Color::DarkGray))
                } else {
                    Span::from(input.to_printable(self.printable_style.one_line()))
                },
            ],
            AppUiState::ConfirmReplacement(_) => vec![Span::from(
                "Press <enter> to write changes, <esc> to cancel.",
            )],
        };

        let mut render_input = |spans| f.render_widget(Paragraph::new(Spans::from(spans)), r);

        // Draw input cursor after rendering input
        if let AppUiState::InputReplacement(input) = &self.ui_state {
            let x_start = r.x + (prefix.len() as u16);
            let x_pos = if input.is_empty() {
                0
            } else {
                spans.last().map(|span| span.width()).unwrap() as u16
            };

            spans.push(Span::styled(
                "    (press <control+s> to accept replacement)",
                Style::default().fg(Color::DarkGray),
            ));

            render_input(spans);
            f.set_cursor(x_start + x_pos, r.y);
        } else {
            render_input(spans);
        }
    }

    fn draw_stats_line<B: Backend>(&mut self, f: &mut Frame<B>, r: Rect) {
        let replacement_count = self
            .list
            .iter()
            .filter_map(|i| {
                if matches!(i.kind, RgMessageKind::Match) {
                    Some(i.replace_count())
                } else {
                    None
                }
            })
            .sum::<usize>();

        // Split the stats line into halves, so we can render left and right aligned portions.
        let hsplit = Layout::default()
            .direction(Direction::Horizontal)
            // NOTE: Length is 10 because the longest `AppUiState.to_span()` is 10 characters.
            .constraints([Constraint::Length(10), Constraint::Min(1)].as_ref())
            .split(r);

        let left_side_items = vec![Spans::from(self.ui_state.to_span())];
        let right_side_items = vec![Spans::from(vec![
            Span::styled(
                format!(" {} ", self.rg_cmdline),
                Style::default().bg(Color::Blue).fg(Color::Black),
            ),
            Span::styled(
                format!(" CtrlChars: {} ", self.printable_style),
                Style::default().bg(Color::Cyan).fg(Color::Black),
            ),
            Span::styled(
                format!(" {}/{} ", replacement_count, self.stats.matches),
                Style::default().bg(Color::Magenta).fg(Color::Black),
            ),
        ])];

        let stats_line_style = Style::default().bg(Color::DarkGray).fg(Color::White);
        f.render_widget(
            Paragraph::new(left_side_items)
                .style(stats_line_style)
                .alignment(Alignment::Left),
            hsplit[0],
        );
        f.render_widget(
            Paragraph::new(right_side_items)
                .style(stats_line_style)
                .alignment(Alignment::Right),
            hsplit[1],
        );
    }

    fn draw_help_view<B: Backend>(&mut self, f: &mut Frame<B>, r: Rect) {
        let title_style = Style::default().fg(Color::Magenta);
        let hsplit = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(r);

        let help_table = Table::new(
            ["[Key]", "[Action]"].iter(),
            vec![
                Row::StyledData(["MODE: ALL"].iter(), title_style),
                Row::Data(["control + b", "move backward one page"].iter()),
                Row::Data(["control + f", "move forward one page"].iter()),
                Row::Data(["control + v", "toggle how control characters are rendered"].iter()),
                Row::Data([].iter()),
                Row::StyledData(["MODE: SELECT"].iter(), title_style),
                Row::Data(["k, up", "move to previous match"].iter()),
                Row::Data(["j, down", "move to next match"].iter()),
                Row::Data(["K, shift + up", "move to previous file"].iter()),
                Row::Data(["J, shift + down", "move to next file"].iter()),
                Row::Data(["space", "toggle selection"].iter()),
                Row::Data(["a, A", "toggle selection for all matches"].iter()),
                Row::Data(["s, S", "toggle selection for whole line"].iter()),
                Row::Data(["enter, r, R", "accept selection"].iter()),
                Row::Data(["q, esc", "quit"].iter()),
                Row::Data(["?", "show help and keybindings"].iter()),
                Row::Data([].iter()),
                Row::StyledData(["MODE: REPLACE"].iter(), title_style),
                Row::Data(["control + s", "accept replacement text"].iter()),
                Row::Data(["esc", "previous mode"].iter()),
                Row::Data([].iter()),
                Row::StyledData(["MODE: CONFIRM"].iter(), title_style),
                Row::Data(["enter", "write replacements to disk"].iter()),
                Row::Data(["q, esc", "previous mode"].iter()),
            ]
            .into_iter(),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled("Keybindings", Style::from(title_style))),
        )
        .header_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .widths(&[Constraint::Length(20), Constraint::Length(50)])
        .column_spacing(1);

        f.render_widget(help_table, hsplit[1]);

        let help_title = Span::styled(format!("{} help", crate_name!()), Style::from(title_style));
        let help_text = self.help_text_state.text(hsplit[0].height as usize);
        let help_text = Text::from(help_text.as_ref());
        let help_paragraph = Paragraph::new(help_text)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(help_title));

        f.render_widget(help_paragraph, hsplit[0]);
    }

    fn get_replacement_text(&self) -> Option<&str> {
        match &self.ui_state {
            AppUiState::InputReplacement(replacement)
            | AppUiState::ConfirmReplacement(replacement) => Some(replacement.as_str()),
            _ => None,
        }
    }

    fn list_indicator(&self) -> String {
        if self.ui_state.is_replacing() {
            " ".repeat(LIST_HIGHLIGHT_SYMBOL.len())
        } else {
            String::from(LIST_HIGHLIGHT_SYMBOL)
        }
    }

    fn list_indicator_width(&self) -> u16 {
        Span::from(self.list_indicator().as_str()).width() as u16
    }

    fn draw_main_view<B: Backend>(&mut self, f: &mut Frame<B>, r: Rect) {
        let ctx = &UiItemContext {
            replacement_text: self.get_replacement_text(),
            printable_style: self.printable_style,
            app_list_state: &self.list_state,
            app_ui_state: &self.ui_state,
            rect: self.main_view_list_rect(f.size()),
        };

        let match_items = self
            .list
            .iter()
            .flat_map(|item| {
                item.to_span_lines(&ctx)
                    .into_iter()
                    .map(|spans| ListItem::new(spans))
                    .collect::<Vec<ListItem>>()
            })
            .collect::<Vec<ListItem>>();

        // TODO: highlight the bg of whole line (not just the text on it), currently not possible
        // See: https://github.com/fdehau/tui-rs/issues/239#issuecomment-657070300
        let indicator_symbol = self.list_indicator();
        let match_list = List::new(match_items)
            .block(Block::default())
            .style(Style::default().fg(Color::White))
            .highlight_symbol(&indicator_symbol);

        f.render_stateful_widget(match_list, r, &mut self.list_state.indicator_mut());
    }

    pub(crate) fn main_view_list_rect(&self, term_size: Rect) -> Rect {
        let Rect {
            x,
            y,
            width,
            height,
        } = self.get_layouts(term_size).0[0];
        let indicator_width = self.list_indicator_width();
        Rect::new(
            x + indicator_width,
            y,
            width.saturating_sub(indicator_width),
            height,
        )
    }
}
