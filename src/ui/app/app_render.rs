/// Rendering for `App`.
use const_format::formatcp;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Color;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table, Wrap};

use crate::model::Printable;
use crate::rg::de::RgMessageKind;
use crate::ui::app::{App, AppUiState};
use crate::ui::render::UiItemContext;
use crate::util::byte_pos_from_char_pos;

const LIST_HIGHLIGHT_SYMBOL: &str = "-> ";
const MINIMUM_WIDTH: u16 = 70;
const MINIMUM_HEIGHT: u16 = 20;
const TOO_SMALL_MESSAGE: &str = formatcp!(
    "Terminal window is too small!
Minimum dimensions are: {}x{}.
Resize your terminal window or press 'esc' or 'q' to quit.",
    MINIMUM_WIDTH,
    MINIMUM_HEIGHT
);

impl App {
    // The UI is:
    // _
    // | - list
    // | - of
    // | - matches
    // | status line (rg command line, matches, replacements, etc)
    // | command line (user input for replacement text, etc)
    // _
    pub fn draw(&mut self, f: &mut Frame) {
        let frame = f.area();
        if self.is_frame_too_small(frame) {
            return self.draw_too_small_view(f, frame);
        }

        let (root_split, stats_and_input_split) = self.get_layouts(frame);
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

        (root_split.to_vec(), stats_and_input_split.to_vec())
    }

    pub(crate) fn is_frame_too_small(&self, frame: Rect) -> bool {
        frame.width < MINIMUM_WIDTH || frame.height < MINIMUM_HEIGHT
    }

    fn draw_too_small_view(&self, f: &mut Frame, r: Rect) {
        let p = Paragraph::new(Text::from(TOO_SMALL_MESSAGE)).wrap(Wrap { trim: false });
        f.render_widget(p, r);
    }

    fn draw_input_line(&mut self, f: &mut Frame, r: Rect) {
        let prefix = "Replacement: ";
        let mut spans = match &self.ui_state {
            AppUiState::Help => vec![Span::from("Viewing Help. Press <esc> or <q> to return...")],
            AppUiState::SelectMatches => vec![Span::from(
                "Select (or deselect) Matches with <space> then press <Enter>. Press <?> for help.",
            )],
            AppUiState::InputReplacement(input, pos) => {
                let mut spans = vec![Span::from(prefix)];
                if input.is_empty() {
                    spans.push(Span::styled("<empty>", self.theme.muted));
                } else {
                    let (before, after) = input.split_at(byte_pos_from_char_pos(input, *pos));
                    let style = self.printable_style.as_one_line();
                    spans.push(Span::from(before.to_printable(style)));
                    spans.push(Span::from(after.to_printable(style)));
                }

                spans
            }
            AppUiState::ConfirmReplacement(_, _) => vec![Span::from(
                "Press <enter> to write changes, <esc> to cancel.",
            )],
        };

        let mut render_input = |spans| f.render_widget(Paragraph::new(Line::from(spans)), r);

        // Draw input cursor after rendering input
        if let AppUiState::InputReplacement(input, _) = &self.ui_state {
            let x_start = r.x + (prefix.len() as u16);
            let x_pos = if input.is_empty() {
                0
            } else {
                spans[spans.len() - 2].width() as u16
            };

            spans.push(Span::styled(
                "    (press <enter> or <C-s> to accept replacement)",
                self.theme.muted,
            ));

            render_input(spans);
            f.set_cursor_position((x_start + x_pos, r.y));
        } else {
            render_input(spans);
        }
    }

    fn draw_stats_line(&mut self, f: &mut Frame, r: Rect) {
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
            // NOTE: Length is 16 to accommodate the `AppUiState.to_span()` (up to 10 characters) plus a count prefix.
            .constraints([Constraint::Length(16), Constraint::Min(1)].as_ref())
            .split(r);

        let mut left_side_spans = vec![self.ui_state.to_span(&self.theme)];
        if let Some(count) = self.count {
            left_side_spans.push(Span::styled(format!(" {} ", count), self.theme.emphasis));
        }
        let left_side_items = vec![Line::from(left_side_spans)];
        let right_side_items = vec![Line::from(vec![
            Span::styled(format!(" {} ", self.rg_cmdline), self.theme.normal),
            Span::styled(
                format!(" CtrlChars: {} ", self.printable_style),
                self.theme.normal,
            ),
            Span::styled(
                format!(" {}/{} ", replacement_count, self.stats.matches),
                self.theme.emphasis,
            ),
        ])];

        f.render_widget(
            Paragraph::new(left_side_items)
                .style(self.theme.status)
                .alignment(Alignment::Left),
            hsplit[0],
        );
        f.render_widget(
            Paragraph::new(right_side_items)
                .style(self.theme.status)
                .alignment(Alignment::Right),
            hsplit[1],
        );
    }

    fn draw_help_view(&mut self, f: &mut Frame, r: Rect) {
        let title_style = self.theme.help_heading;
        let hsplit = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(r);

        let help_table = Table::new(
            vec![
                Row::new(vec!["MODE: ALL"]).style(title_style),
                Row::new(vec!["control + b", "move backward one page"]),
                Row::new(vec!["control + f", "move forward one page"]),
                Row::new(vec![
                    "control + v",
                    "toggle how control characters are rendered",
                ])
                .bottom_margin(1),
                Row::new(vec!["MODE: SELECT"]).style(title_style),
                Row::new(vec!["k, up", "move to previous match"]),
                Row::new(vec!["j, down", "move to next match"]),
                Row::new(vec!["K, shift + up", "move to previous file"]),
                Row::new(vec!["J, shift + down", "move to next file"]),
                Row::new(vec!["g", "move to first match"]),
                Row::new(vec!["G", "move to last match"]),
                Row::new(vec!["<n><motion>", "repeat motion n times (e.g. 2j)"]),
                Row::new(vec!["space", "toggle selection"]),
                Row::new(vec!["a, A", "toggle selection for all matches"]),
                Row::new(vec!["s, S", "toggle selection for whole line"]),
                Row::new(vec!["v", "invert section for the current item"]),
                Row::new(vec!["V", "invert section for all items"]),
                Row::new(vec!["enter, r, R", "accept selection"]),
                Row::new(vec!["q, esc", "quit"]),
                Row::new(vec!["?", "show help and keybindings"]).bottom_margin(1),
                Row::new(vec!["MODE: REPLACE"]).style(title_style),
                Row::new(vec!["control + s", "accept replacement text"]),
                Row::new(vec!["esc", "previous mode"]).bottom_margin(1),
                Row::new(vec!["MODE: CONFIRM"]).style(title_style),
                Row::new(vec!["enter", "write replacements to disk"]),
                Row::new(vec!["q, esc", "previous mode"]),
            ],
            [Constraint::Length(20), Constraint::Length(50)],
        )
        .header(
            Row::new(vec!["[Key]", "[Action]"])
                .style(self.theme.help_keys)
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled("Keybindings", title_style)),
        )
        .column_spacing(1);

        f.render_widget(help_table, hsplit[1]);

        let help_title = Span::styled(format!("{} help", env!("CARGO_PKG_NAME")), title_style);
        let help_text = self.help_text_state.text(hsplit[0].height as usize);
        let help_text = Text::from(help_text.as_str());
        let help_paragraph = Paragraph::new(help_text)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(help_title));

        f.render_widget(help_paragraph, hsplit[0]);
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

    fn draw_main_view(&mut self, f: &mut Frame, r: Rect) {
        let list_rect = self.main_view_list_rect(f.area());
        let indicator_symbol = self.list_indicator();

        // For performance with large match sets, we only send a single "window"'s
        // worth of lines to the terminal. For all our calculations, we use a line
        // count (to generate the list, define the window, determine the position
        // of the indicator, etc) and only when we send it to the rendering library
        // do we adjust it for the window. See `App::update_indicator`.
        let window_height = list_rect.height as usize;
        let window_start = self.list_state.window_start();
        let window_end = window_start + window_height;

        let absolute_indicator_idx = window_start + self.list_state.indicator().selected().unwrap_or(0);

        let ctx = &UiItemContext {
            capture_pattern: self.capture_pattern.as_ref(),
            replacement_text: self.ui_state.user_replacement_text(),
            printable_style: self.printable_style,
            app_list_state: &self.list_state,
            app_ui_state: &self.ui_state,
            theme: self.theme,
            list_rect,
        };

        // iterate over all our items and collect only those that will be in the visible
        // window region of the list (skipping all the others)
        let mut match_items = vec![];
        let mut curr_height = 0;
        for item in self.list.iter_mut() {
            // we've passed the visible region
            if curr_height > window_end {
                break;
            }

            let line_count = item.line_count(list_rect.width, self.printable_style);
            let lines = item.to_span_lines(ctx);

            for (i, line) in lines.into_iter().enumerate() {
                let absolute_line_idx = curr_height + i;

                // skip if above visible window
                if absolute_line_idx < window_start {
                    continue;
                }

                let diff = absolute_line_idx as i32 - absolute_indicator_idx as i32;
                let line_number_text = if diff == 0 {
                    format!("{:>3} ", item.index + 1)
                } else {
                    format!("{:>3} ", diff.abs())
                };
                let line_number_style = if diff == 0 {
                    self.theme.normal.fg(Color::Yellow)
                } else {
                    self.theme.muted
                };

                let mut spans = line.spans;
                spans.insert(0, Span::styled(line_number_text, line_number_style));
                match_items.push(ListItem::new(Line::from(spans)));
            }

            curr_height += line_count;
        }

        // TODO: highlight the bg of whole line (not just the text on it), currently not possible
        // See: https://github.com/fdehau/tui-rs/issues/239#issuecomment-657070300
        let match_list = List::new(match_items)
            .block(Block::default())
            .style(self.theme.normal)
            .highlight_symbol(indicator_symbol.as_str());

        f.render_stateful_widget(match_list, r, self.list_state.indicator_mut());
    }

    pub(crate) fn main_view_list_rect(&self, term_size: Rect) -> Rect {
        let Rect {
            x,
            y,
            width,
            height,
        } = self.get_layouts(term_size).0[0];
        let indicator_width = self.list_indicator_width();
        let line_number_width = 4;
        Rect::new(
            x + indicator_width + line_number_width,
            y,
            width.saturating_sub(indicator_width + line_number_width),
            height,
        )
    }
}
