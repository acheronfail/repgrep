use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::cursor::Show;
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::model::{RegexConfig, ReplacementCriteria, capture_pattern};
use crate::rg::de::RgMessage;
use crate::ui::app::{App, AppState};
use crate::ui::theme::Theme;

const FALLBACK_MESSAGE: &str = r#"
You may continue to use repgrep, however capturing groups will be ignored for this session."#;

pub struct Tui {
    term: Terminal<CrosstermBackend<Stdout>>,
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

        Ok(Tui { term })
    }

    fn draw_message_box(&mut self, title: impl AsRef<str>, body: impl AsRef<str>) -> Result<()> {
        self.term.clear()?;
        let theme = Theme::default();
        self.term.draw(|f| {
            let block = Block::default()
                .style(theme.error)
                .borders(Borders::ALL)
                .title(title.as_ref());

            // TODO: check minimum size?
            let frame = f.area();

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
                    .style(theme.normal),
                p_frame,
            );
        })?;

        // display until user acknowledges
        loop {
            match event::read()? {
                Event::Key(key)
                    if matches!(key.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q')) =>
                {
                    break;
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
        patterns: &[String],
        regex_config: &RegexConfig,
        fixed_strings: bool,
        replacement: Option<String>,
    ) -> Result<Option<ReplacementCriteria>> {
        let capture_pattern = match capture_pattern(patterns, regex_config, fixed_strings) {
            Ok(capture_pattern) => capture_pattern,
            Err(error) => {
                self.draw_message_box(
                    "Unsupported Arguments!",
                    format!("{error}\n\n{FALLBACK_MESSAGE}"),
                )?;
                None
            }
        };

        // main app event loop
        let mut app = App::new(capture_pattern, rg_cmdline, rg_messages, replacement);
        loop {
            let before_draw = Instant::now();
            self.term.draw(|f| app.draw(f))?;

            // If drawing to the terminal is slow, flush all keyboard events so they're not buffered.
            // (Otherwise with very slow updates, the user has to wait for all keyboard events to be processed
            // before being able to quit the app, etc).
            if before_draw.elapsed() > Duration::from_millis(20) {
                while event::poll(Duration::ZERO)? {
                    event::read()?;
                }
            }

            let event = event::read()?;
            let term_size = self.term.get_frame().area();
            app.on_event(term_size, event)?;

            match app.state {
                AppState::Running => continue,
                AppState::Cancelled => return Ok(None),
                AppState::Complete => return Ok(Some(app.get_replacement_criteria()?)),
            }
        }
    }

    pub fn restore_terminal() -> Result<()> {
        terminal::disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen, Show)?;

        Ok(())
    }
}
