use std::collections::VecDeque;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use either::Either;
use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Block, List, ListState, Paragraph, Text};
use tui::Frame;

use crate::cli::Args;
use crate::rg::de::{ArbitraryData, RgMessageType, Stats};

fn clamp(val: usize, min: usize, max: usize) -> usize {
  if val <= min {
    min
  } else if val >= max {
    max
  } else {
    val
  }
}

struct Item {
  pub rg_type: RgMessageType,
  pub is_begin: bool,
  pub is_context: bool,
  pub is_match: bool,
  pub is_end: bool,
  pub should_replace: bool,
}

impl Item {
  pub fn new(rg_type: RgMessageType) -> Item {
    let (is_begin, is_context, is_match, is_end) = match rg_type {
      RgMessageType::Begin { .. } => (true, false, false, false),
      RgMessageType::Match { .. } => (false, false, true, false),
      RgMessageType::Context { .. } => (false, true, false, false),
      RgMessageType::End { .. } => (false, true, false, true),
      _ => (false, false, false, false),
    };

    Item {
      rg_type,
      is_begin,
      is_context,
      is_match,
      is_end,
      should_replace: true,
    }
  }

  pub fn path(&self) -> &ArbitraryData {
    match &self.rg_type {
      RgMessageType::Begin { path } => path,
      RgMessageType::Context { path, .. } => path,
      RgMessageType::Match { path, .. } => path,
      RgMessageType::End { path, .. } => path,
      unexpected_type => panic!(
        "Unexpected enum variant, got {:?} and expected only Begin, Context, Match or End!",
        unexpected_type
      ),
    }
  }

  pub fn is_selectable(&self) -> bool {
    self.is_begin || self.is_match
  }

  // TODO: left-pad line numbers?
  pub fn to_text(&self) -> Text {
    // TODO: color line number, currently not possible
    // See: https://github.com/fdehau/tui-rs/issues/315
    let lines_as_string = |lines: &ArbitraryData, line_number: &Option<usize>| {
      let mut s = lines.lossy_utf8();
      if let Some(number) = line_number {
        s = format!("{}:{}", number, s);
      }

      s
    };

    // TODO: handle non-UTF-8 text
    match &self.rg_type {
      RgMessageType::Begin { path, .. } => Text::styled(
        format!("file: {}", path.lossy_utf8()),
        Style::default().fg(Color::Magenta),
      ),
      RgMessageType::Context {
        lines, line_number, ..
      } => Text::styled(
        lines_as_string(lines, line_number),
        Style::default().fg(Color::DarkGray),
      ),
      RgMessageType::Match {
        lines, line_number, ..
      } => {
        // TODO: highlight matches on line, currently not possible
        // See: https://github.com/fdehau/tui-rs/issues/315
        let mut style = Style::default();
        if !self.should_replace {
          style = style.fg(Color::Red);
        }

        Text::styled(lines_as_string(lines, line_number), style)
      }
      RgMessageType::End { .. } => Text::raw(""),
      unexpected_type => panic!(
        "Unexpected enum variant, got {:?} and expected only Context or Match!",
        unexpected_type
      ),
    }
  }
}

#[derive(Debug, Eq, PartialEq)]
enum Movement {
  Prev,
  Next,
  Forward(u16),
  Backward(u16),
}

pub struct App {
  pub should_quit: bool,

  rg_cmdline: String,
  stats: Stats,

  list: Vec<Item>,
  list_state: ListState,
}

impl App {
  pub fn new(args: &Args, mut rg_results: VecDeque<RgMessageType>) -> App {
    let mut list = vec![];
    let mut maybe_stats = None;
    while let Some(rg_type) = rg_results.pop_front() {
      match rg_type {
        RgMessageType::Summary { stats, .. } => {
          maybe_stats = Some(stats);
          // NOTE: there should only be one RgMessageType::Summary, and it should be the last item.
          break;
        }
        t => list.push(Item::new(t)),
      }
    }

    let mut list_state = ListState::default();
    list_state.select(Some(0));

    App {
      rg_cmdline: format!("rg {}", args.rg_args.join(" ")),
      should_quit: false,
      stats: maybe_stats.unwrap(),
      list_state,
      list,
    }
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

  // rg cmdline
  // - list
  // - of
  // - matches
  // repgrep status line (how many to replace, etc)
  // repgrep command line (enter text to replace here, etc)
  pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
    let (root_split, stats_and_input_split) = self.get_layouts(f.size());
    self.draw_match_list(f, root_split[0]);
    self.draw_stats_line(f, stats_and_input_split[0]);
    self.draw_input_line(f, stats_and_input_split[1]);
  }

  fn draw_input_line<B: Backend>(&mut self, f: &mut Frame<B>, r: Rect) {
    // TODO: user input for replacement string
    let text = Text::raw("> TODO...");
    f.render_widget(Paragraph::new([text].iter()), r);
  }

  fn draw_stats_line<B: Backend>(&mut self, f: &mut Frame<B>, r: Rect) {
    let replacement_count = self
      .list
      .iter()
      .filter(|i| i.is_match && i.should_replace)
      .count();

    let text = Text::raw(format!(
      "rg: {}, Matches: {}, Replacements: {}",
      self.rg_cmdline, self.stats.matches, replacement_count
    ));

    f.render_widget(
      Paragraph::new([text].iter())
        .style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .alignment(Alignment::Right),
      r,
    );
  }

  fn draw_match_list<B: Backend>(&mut self, f: &mut Frame<B>, r: Rect) {
    let match_items = self.list.iter().map(|item| item.to_text());

    let curr_item = &self.list[self.curr_pos()];
    let highlight_style = Style::default().fg(if curr_item.is_match {
      if curr_item.should_replace {
        Color::Yellow
      } else {
        Color::Red
      }
    } else if curr_item.is_begin {
      Color::Magenta
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

    f.render_stateful_widget(match_list, r, &mut self.list_state)
  }

  fn curr_pos(&self) -> usize {
    self.list_state.selected().unwrap_or(0)
  }

  // TODO: support selecting submatches
  fn move_pos(&mut self, direction: Movement) {
    let iterator = self.list.iter().enumerate();
    let iterator = match direction {
      Movement::Prev | Movement::Backward(_) => Either::Left(iterator.rev()),
      Movement::Next | Movement::Forward(_) => Either::Right(iterator),
    };

    let current = self.curr_pos();
    let (skip, default) = match direction {
      Movement::Prev => (self.list.len().saturating_sub(current), 0),
      Movement::Backward(n) => (
        self
          .list
          .len()
          .saturating_sub(current.saturating_sub(n as usize)),
        0,
      ),

      Movement::Next => (current, self.list.len() - 1),
      Movement::Forward(n) => (current + (n as usize), self.list.len() - 1),
    };

    let pos = iterator
      .skip(skip)
      .find_map(|(i, item)| {
        let is_valid_next = match direction {
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

    self
      .list_state
      .select(Some(clamp(pos, 0, self.list.len() - 1)));
  }

  pub fn on_event(&mut self, term_size: Rect, event: Event) -> Result<()> {
    if let Event::Key(key) = event {
      // TODO: toggle between inputting "replacement" text, and moving and selecting?
      //    maybe select first, and on enter switch to entering replacement string (esc for back)
      //    on enter again, apply changes

      // CONTROL+KEY
      if key.modifiers.contains(KeyModifiers::CONTROL) {
        let (root_split, _) = self.get_layouts(term_size);
        let list_height = root_split[0].height;

        match key.code {
          KeyCode::Char('b') => self.move_pos(Movement::Backward(list_height)),
          KeyCode::Char('f') => self.move_pos(Movement::Forward(list_height)),
          _ => {}
        }
      } else {
        match key.code {
          KeyCode::Char('q') => self.should_quit = true,
          KeyCode::Up | KeyCode::Char('k') => self.move_pos(Movement::Prev),
          KeyCode::Down | KeyCode::Char('j') => self.move_pos(Movement::Next),
          // KeyCode::Char('x') => {}, // TODO: delete item
          KeyCode::Char(' ') => {
            let curr_pos = self.curr_pos();

            // If Match item, toggle replace.
            if self.list[curr_pos].is_match {
              let selected_item = &mut self.list[curr_pos];
              selected_item.should_replace = !selected_item.should_replace;
            }

            // If Begin item, toggle all matches in it.
            if self.list[curr_pos].is_begin {
              let mut items_to_toggle: Vec<_> = self
                .list
                .iter_mut()
                .skip(curr_pos)
                .take_while(|i| !i.is_end)
                .filter(|i| i.is_match)
                .collect();

              let should_replace = items_to_toggle.iter().all(|i| !i.should_replace);
              for item in items_to_toggle.iter_mut() {
                item.should_replace = should_replace;
              }
            }
          }
          _ => {}
        }
      }
    }

    Ok(())
  }
}
