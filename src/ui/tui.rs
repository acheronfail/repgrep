use std::io::{self, Stdout};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use regex::bytes::Regex;
use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, Paragraph, Wrap};
use tui::{backend::CrosstermBackend, Terminal};

use crate::model::ReplacementCriteria;
use crate::rg::de::RgMessage;
use crate::ui::app::{App, AppState};

const FALLBACK_MESSAGE: &str = r#"
You may continue to use repgrep, however capturing groups will be ignored for this session."#;

pub struct Tui {
    term: Terminal<CrosstermBackend<Stdout>>,
    rx: Receiver<Event>,
}

impl Tui {
    pub fn new() -> Result<Tui> {
        terminal::enable_raw_mode()?;

        let mut stdout = io::stdout();
        // NOTE: must match options in `Self::restore_terminal()`
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let mut term = Terminal::new(backend)?;
        term.hide_cursor()?;

        // Setup input handling
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || loop {
            match tx.send(event::read().expect("failed to read event from terminal")) {
                Ok(_) => {}
                Err(e) => log::warn!("failed to send event to the main thread: {}", e),
            }
        });

        term.clear()?;

        Ok(Tui { term, rx })
    }

    fn draw_message_box(&mut self, title: impl AsRef<str>, body: impl AsRef<str>) -> Result<()> {
        self.term.clear()?;
        self.term.draw(|f| {
            let block = Block::default()
                .style(Style::default().fg(Color::Red))
                .borders(Borders::ALL)
                .title(title.as_ref());

            // TODO: check minimum size?
            let frame = f.size();

            // calculate message box size
            let body = body.as_ref();
            let body_lines = body.lines().count();
            let block_frame = Rect::new(
                frame.width / 4,
                frame.height / 4,
                frame.width / 2,
                u16::min(
                    frame.height / 2,
                    // +6 accounting for borders and padding
                    6 + body_lines as u16,
                ),
            );

            // calculate inner paragraph bounds
            let inner_frame = block.inner(block_frame);
            let p_frame = Rect::new(
                inner_frame.x.saturating_add(1),
                inner_frame.y.saturating_add(1),
                inner_frame.width.saturating_sub(1),
                inner_frame.height.saturating_sub(1),
            );

            f.render_widget(block, block_frame);
            f.render_widget(
                Paragraph::new(body)
                    .wrap(Wrap { trim: true })
                    .style(Style::default().fg(Color::White)),
                p_frame,
            );
        })?;

        // display until user acknowledges
        loop {
            match self.rx.recv() {
                Ok(Event::Key(key))
                    if matches!(key.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q')) =>
                {
                    break
                }

                _ => continue,
            }
        }

        self.term.clear()?;
        Ok(())
    }

    pub fn start(
        mut self,
        rg_cmdline: String,
        rg_messages: Vec<RgMessage>,
        patterns: Vec<&str>,
    ) -> Result<Option<ReplacementCriteria>> {
        // Parse patterns into `Regex` structs
        let patterns = patterns
            .into_iter()
            .map(|p| Regex::new(p))
            .collect::<Result<Vec<_>, _>>();

        // Check if we should be performing replacements with capturing groups.
        let capture_pattern = match patterns {
            // pattern with capturing group passed, and we only have one
            Ok(mut one) if one.len() == 1 => {
                // SAFETY: we just checked for length in this match
                (one[0].captures_len() > 1).then_some(one.pop().unwrap())
            }
            // many patterns passed, and one had a capturing group
            // all regex's have at least one capturing group, see: https://docs.rs/regex/1.8.4/regex/struct.Captures.html#method.len
            Ok(many) if many.iter().any(|re| re.captures_len() > 1) => {
                self.draw_message_box(
                    "Unsupported Arguments!",
                    format!(
                        "{}\n\nPatterns:\n\n{patterns}\n\n{fallback}",
                        "Either pass a single pattern with capturing groups, or many patterns without capturing groups.",
                        patterns = many
                            .iter()
                            .map(|re| format!("  - {}", re.as_str()))
                            .collect::<Vec<_>>()
                            .join("\n"),
                            fallback = FALLBACK_MESSAGE
                    ),
                )?;

                None
            }
            // many patterns passed, none had capturing groups
            Ok(_) => None,
            // failed to parse patterns
            Err(e) => {
                self.draw_message_box(
                    "Error!",
                    format!(
                        "{}\n\nError: {}\n\n{fallback}",
                        "Failed to pass patterns!",
                        e,
                        fallback = FALLBACK_MESSAGE
                    ),
                )?;

                None
            }
        };

        // main app event loop
        let mut app = App::new(capture_pattern, rg_cmdline, rg_messages);
        let mut term = self.term;
        loop {
            let before_draw = Instant::now();
            term.draw(|mut f| app.draw(&mut f))?;

            // If drawing to the terminal is slow, flush all keyboard events so they're not buffered.
            // (Otherwise with very slow updates, the user has to wait for all keyboard events to be processed
            // before being able to quit the app, etc).
            if before_draw.elapsed() > Duration::from_millis(20) {
                while let Ok(_) = self.rx.try_recv() {}
            }

            let event = self.rx.recv()?;
            let term_size = term.get_frame().size();
            app.on_event(term_size, event)?;

            match app.state {
                AppState::Running => continue,
                AppState::Cancelled => return Ok(None),
                AppState::Complete => return Ok(Some(app.get_replacement_criteria()?)),
            }
        }
    }

    pub fn restore_terminal() -> Result<()> {
        let backend = CrosstermBackend::new(io::stdout());
        let mut term = Terminal::new(backend)?;

        terminal::disable_raw_mode()?;
        execute!(term.backend_mut(), LeaveAlternateScreen)?;
        term.show_cursor()?;
        term.clear()?;
        term.set_cursor(0, 0)?;

        Ok(())
    }
}
