mod state;

use std::collections::VecDeque;

use anyhow::Result;
use clap::crate_name;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use either::Either;
use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, ListState, Paragraph, Row, Table, Text};
use tui::Frame;

use crate::cli::Args;
use crate::model::{Item, Movement, ReplacementCriteria};
use crate::rg::de::{RgMessage, RgMessageKind, Stats};
use crate::util::clamp;
pub use state::AppState;
use state::AppUiState;

const HELP_TEXT: &str = include_str!("../../../doc/help.txt");

pub struct App {
    pub state: AppState,

    rg_cmdline: String,
    stats: Stats,
    list: Vec<Item>,
    list_state: ListState,
    ui_state: AppUiState,
}

// General impl.
impl App {
    pub fn new(args: &Args, mut rg_results: VecDeque<RgMessage>) -> App {
        let mut list = vec![];
        let mut maybe_stats = None;
        while let Some(rg_type) = rg_results.pop_front() {
            match rg_type {
                RgMessage::Summary { stats, .. } => {
                    maybe_stats = Some(stats);
                    // NOTE: there should only be one RgMessage::Summary, and it should be the last item.
                    break;
                }
                t => list.push(Item::new(t)),
            }
        }

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        App {
            state: AppState::Running,

            rg_cmdline: format!("rg {}", args.rg_args.join(" ")),
            stats: maybe_stats.expect("failed to find RgMessage::Summary from rg!"),
            list_state,
            list,
            ui_state: AppUiState::SelectMatches,
        }
    }
}

// Rendering.
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
        let text_items = match &self.ui_state {
            AppUiState::Help => vec![Text::raw("Viewing Help. Press <esc> or <q> to return...")],
            AppUiState::SelectMatches => vec![Text::raw(
                "Select (or deselect) Matches with <space> then press <Enter>. Press <?> for help.",
            )],
            AppUiState::InputReplacement(input) => vec![
                Text::raw("Replacement: "),
                if input.is_empty() {
                    Text::styled("<empty>", Style::default().fg(Color::DarkGray))
                } else {
                    Text::raw(input)
                },
            ],
            AppUiState::ConfirmReplacement(_) => vec![Text::raw(
                "Press <enter> to write changes, <esc> to cancel.",
            )],
        };

        f.render_widget(Paragraph::new(text_items.iter()), r);
    }

    fn draw_stats_line<B: Backend>(&mut self, f: &mut Frame<B>, r: Rect) {
        let replacement_count = self
            .list
            .iter()
            .filter_map(|i| {
                if matches!(i.kind, RgMessageKind::Match) && i.should_replace {
                    Some(i.match_count())
                } else {
                    None
                }
            })
            .sum::<usize>();

        // Split the stats line into halves, so we can render left and right aligned portions.
        let hsplit = Layout::default()
            .direction(Direction::Horizontal)
            // NOTE: Length is 10 because the longest `AppUiState.to_text()` is 10 characters.
            .constraints([Constraint::Length(10), Constraint::Min(1)].as_ref())
            .split(r);

        let left_side_items = [self.ui_state.to_text()];
        let right_side_items = [
            Text::styled(
                format!(" {} ", self.rg_cmdline),
                Style::default().bg(Color::Blue).fg(Color::Black),
            ),
            Text::styled(
                format!(" Matches: {} ", self.stats.matches),
                Style::default().bg(Color::Cyan).fg(Color::Black),
            ),
            Text::styled(
                format!(" Replacements: {} ", replacement_count),
                Style::default().bg(Color::Magenta).fg(Color::Black),
            ),
        ];

        let stats_line_style = Style::default().bg(Color::DarkGray).fg(Color::White);
        f.render_widget(
            Paragraph::new(left_side_items.iter())
                .style(stats_line_style)
                .alignment(Alignment::Left),
            hsplit[0],
        );
        f.render_widget(
            Paragraph::new(right_side_items.iter())
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
                Row::Data([].iter()),
                Row::StyledData(["MODE: SELECT"].iter(), title_style),
                Row::Data(["k, up", "move to previous match"].iter()),
                Row::Data(["j, down", "move to next match"].iter()),
                Row::Data(["K, shift + up", "move to previous file"].iter()),
                Row::Data(["J, shift + down", "move to next file"].iter()),
                Row::Data(["space", "toggle selection"].iter()),
                Row::Data(["a, A", "toggle selection for all matches"].iter()),
                Row::Data(["enter, r, R", "accept selection"].iter()),
                Row::Data(["q, esc", "quit"].iter()),
                Row::Data(["?", "show help and keybindings"].iter()),
                Row::Data([].iter()),
                Row::StyledData(["MODE: REPLACE"].iter(), title_style),
                Row::Data(["enter", "accept replacement text"].iter()),
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
                .title("Keybindings")
                .title_style(title_style),
        )
        .header_style(Style::default().fg(Color::Yellow).modifier(Modifier::BOLD))
        .widths(&[Constraint::Length(20), Constraint::Length(50)])
        .column_spacing(1);

        f.render_widget(help_table, hsplit[1]);

        let help_title = format!("{} help", crate_name!());
        let help_text = [Text::raw(HELP_TEXT)];
        let help_paragraph = Paragraph::new(help_text.iter()).wrap(true).block(
            Block::default()
                .borders(Borders::ALL)
                .title(&help_title)
                .title_style(title_style),
        );

        f.render_widget(help_paragraph, hsplit[0]);
    }

    fn draw_main_view<B: Backend>(&mut self, f: &mut Frame<B>, r: Rect) {
        let replacement = match &self.ui_state {
            AppUiState::InputReplacement(replacement)
            | AppUiState::ConfirmReplacement(replacement) => Some(if replacement.is_empty() {
                "<empty>"
            } else {
                replacement
            }),
            _ => None,
        };

        let match_items = self.list.iter().map(|item| item.to_text(replacement));

        let curr_item = &self.list[self.curr_pos()];
        let highlight_style =
            Style::default().fg(if matches!(curr_item.kind, RgMessageKind::Match) {
                if curr_item.should_replace {
                    Color::Yellow
                } else {
                    Color::Red
                }
            } else if matches!(curr_item.kind, RgMessageKind::Begin) {
                Color::Yellow
            } else {
                Color::DarkGray
            });

        // TODO: highlight the whole line (not just the text on it), currently not possible
        // See: https://github.com/fdehau/tui-rs/issues/239
        let match_list = List::new(match_items)
            .block(Block::default())
            .style(Style::default().fg(Color::White))
            .highlight_symbol("-> ")
            .highlight_style(highlight_style);

        f.render_stateful_widget(match_list, r, &mut self.list_state);
    }

    fn list_height(&self, term_size: Rect) -> u16 {
        let (root_split, _) = self.get_layouts(term_size);
        root_split[0].height
    }
}

// Event Handling.
impl App {
    pub fn on_event(&mut self, term_size: Rect, event: Event) -> Result<()> {
        if let Event::Key(key) = event {
            // Common Ctrl+Key scroll keybindings that apply to multiple modes.
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                let did_handle_key = match &self.ui_state {
                    AppUiState::SelectMatches
                    | AppUiState::InputReplacement(_)
                    | AppUiState::ConfirmReplacement(_) => match key.code {
                        KeyCode::Char('b') => {
                            self.move_pos(Movement::Backward(self.list_height(term_size)));
                            true
                        }
                        KeyCode::Char('f') => {
                            self.move_pos(Movement::Forward(self.list_height(term_size)));
                            true
                        }
                        _ => false,
                    },
                    _ => false,
                };

                // If a key was handled then stop processing any other events.
                if did_handle_key {
                    return Ok(());
                }
            }

            match &self.ui_state {
                AppUiState::ConfirmReplacement(replacement) => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.ui_state = AppUiState::InputReplacement(replacement.to_owned())
                    }
                    KeyCode::Enter => {
                        self.state = AppState::Complete(ReplacementCriteria::new(
                            replacement,
                            self.list.clone(),
                        ));
                    }
                    _ => {}
                },
                AppUiState::Help => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => self.ui_state = AppUiState::SelectMatches,
                    _ => {}
                },
                AppUiState::SelectMatches => {
                    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                            self.move_pos(if shift {
                                Movement::PrevFile
                            } else {
                                Movement::Prev
                            })
                        }
                        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                            self.move_pos(if shift {
                                Movement::NextFile
                            } else {
                                Movement::Next
                            })
                        }
                        KeyCode::Char(' ') => self.toggle_item(),
                        KeyCode::Char('a') | KeyCode::Char('A') => self.toggle_all_items(),
                        KeyCode::Esc | KeyCode::Char('q') => self.state = AppState::Cancelled,
                        KeyCode::Char('?') => self.ui_state = AppUiState::Help,
                        KeyCode::Enter | KeyCode::Char('r') | KeyCode::Char('R') => {
                            self.ui_state = AppUiState::InputReplacement(String::new())
                        }
                        _ => {}
                    }
                }
                AppUiState::InputReplacement(ref input) => match key.code {
                    KeyCode::Char(c) => {
                        let mut new_input = String::from(input);
                        new_input.push(c);
                        self.ui_state = AppUiState::InputReplacement(new_input);
                    }
                    KeyCode::Backspace | KeyCode::Delete => {
                        let new_input = if !input.is_empty() {
                            String::from(input)[..input.len() - 1].to_owned()
                        } else {
                            String::new()
                        };
                        self.ui_state = AppUiState::InputReplacement(new_input);
                    }
                    KeyCode::Esc => self.ui_state = AppUiState::SelectMatches,
                    KeyCode::Enter => {
                        self.ui_state = AppUiState::ConfirmReplacement(input.to_owned())
                    }
                    _ => {}
                },
            }
        }

        Ok(())
    }

    fn curr_pos(&self) -> usize {
        self.list_state.selected().unwrap_or(0)
    }

    // TODO: support selecting submatches
    fn move_pos(&mut self, direction: Movement) {
        let iterator = self.list.iter().enumerate();
        let iterator = match direction {
            Movement::Prev | Movement::PrevFile | Movement::Backward(_) => {
                Either::Left(iterator.rev())
            }
            Movement::Next | Movement::NextFile | Movement::Forward(_) => Either::Right(iterator),
        };

        let current = self.curr_pos();
        let (skip, default) = match direction {
            Movement::Prev | Movement::PrevFile => (self.list.len().saturating_sub(current), 0),
            Movement::Backward(n) => (
                self.list
                    .len()
                    .saturating_sub(current.saturating_sub(n as usize)),
                0,
            ),

            Movement::Next | Movement::NextFile => (current, self.list.len() - 1),
            Movement::Forward(n) => (current + (n as usize), self.list.len() - 1),
        };

        let pos = iterator
            .skip(skip)
            .find_map(|(i, item)| {
                let is_valid_next = match direction {
                    Movement::PrevFile => i < current && matches!(item.kind, RgMessageKind::Begin),
                    Movement::NextFile => i > current && matches!(item.kind, RgMessageKind::Begin),
                    Movement::Prev | Movement::Backward(_) => i < current,
                    Movement::Next | Movement::Forward(_) => i > current,
                };

                if is_valid_next && item.is_selectable() {
                    Some(i)
                } else {
                    None
                }
            })
            .unwrap_or(default);

        self.list_state
            .select(Some(clamp(pos, 0, self.list.len() - 1)));
    }

    fn toggle_item(&mut self) {
        let curr_pos = self.curr_pos();

        // If Match item, toggle replace.
        if matches!(self.list[curr_pos].kind, RgMessageKind::Match) {
            let selected_item = &mut self.list[curr_pos];
            selected_item.should_replace = !selected_item.should_replace;
        }

        // If Begin item, toggle all matches in it.
        if matches!(self.list[curr_pos].kind, RgMessageKind::Begin) {
            let mut items_to_toggle: Vec<_> = self
                .list
                .iter_mut()
                .skip(curr_pos)
                .take_while(|i| i.kind != RgMessageKind::End)
                .filter(|i| i.kind == RgMessageKind::Match)
                .collect();

            let should_replace = items_to_toggle.iter().all(|i| !i.should_replace);
            for item in items_to_toggle.iter_mut() {
                item.should_replace = should_replace;
            }
        }
    }

    fn toggle_all_items(&mut self) {
        let should_replace = self.list.iter().all(|i| !i.should_replace);
        for item in self.list.iter_mut() {
            item.should_replace = should_replace;
        }
    }
}
