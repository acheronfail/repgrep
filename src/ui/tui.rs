use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use tui::{backend::CrosstermBackend, Terminal};

use crate::model::ReplacementCriteria;
use crate::rg::de::RgMessage;
use crate::ui::app::{App, AppState};

pub struct Tui {
    app: App,
}

impl Tui {
    pub fn new(rg_cmdline: String, rg_messages: Vec<RgMessage>) -> Tui {
        Tui {
            app: App::new(rg_cmdline, rg_messages),
        }
    }

    pub fn start(mut self) -> Result<Option<ReplacementCriteria>> {
        terminal::enable_raw_mode()?;

        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

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

        loop {
            let before_draw = Instant::now();
            let term_size = term.get_frame().size();
            term.draw(|mut f| self.app.draw(&mut f))?;

            // If drawing to the terminal is slow, flush all keyboard events so they're not buffered.
            // (Otherwise with very slow updates, the user has to wait for all keyboard events to be processed
            // before being able to quit the app, etc).
            if before_draw.elapsed() > Duration::from_millis(20) {
                while let Ok(_) = rx.try_recv() {}
            }

            let event = rx.recv()?;
            self.app.on_event(term_size, event)?;

            match self.app.state {
                AppState::Running => continue,
                AppState::Cancelled => return Ok(None),
                AppState::Complete(replacement_criteria) => return Ok(Some(replacement_criteria)),
            }
        }
    }

    pub fn restore_terminal() -> Result<()> {
        let backend = CrosstermBackend::new(io::stdout());
        let mut term = Terminal::new(backend)?;

        terminal::disable_raw_mode()?;
        execute!(
            term.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        term.show_cursor()?;
        term.clear()?;
        term.set_cursor(0, 0)?;

        Ok(())
    }
}
