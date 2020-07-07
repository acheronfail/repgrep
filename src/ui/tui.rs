use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;

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
    pub fn new(rg_cmdline: String, rg_results: VecDeque<RgMessage>) -> Tui {
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
            tx.send(event::read().unwrap()).unwrap();
        });

        term.clear()?;

        loop {
            term.draw(|mut f| self.app.draw(&mut f))?;

            let event = rx.recv()?;
            self.app.on_event(term.size()?, event)?;

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
