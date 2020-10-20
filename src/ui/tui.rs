use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;

use anyhow::{anyhow, Result};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use tui::{backend::CrosstermBackend, Terminal};

use crate::model::ReplacementCriteria;
use crate::rg::de::RgMessage;
use crate::ui::app::{App, AppState};

const MINIMUM_WIDTH: u16 = 40;
const MINIMUM_HEIGHT: u16 = 40;

pub struct Tui {
    app: App,
}

impl Tui {
    pub fn new(rg_cmdline: String, rg_results: Vec<RgMessage>) -> Tui {
        Tui {
            app: App::new(rg_cmdline, rg_results),
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
            let term_size = term.size()?;
            if term_size.width < MINIMUM_WIDTH || term_size.height < MINIMUM_HEIGHT {
                return Err(anyhow!(
                    "Minimum terminal dimensions are {}x{}!",
                    MINIMUM_WIDTH,
                    MINIMUM_HEIGHT
                ));
            }

            term.draw(|mut f| self.app.draw(&mut f))?;

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
