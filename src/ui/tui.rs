use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;

use anyhow::Result;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use tui::{backend::CrosstermBackend, Terminal};

use crate::cli::Args;
use crate::rg::de::RgMessageType;
use crate::ui::app::App;

pub struct Tui {
  app: App,
}

impl Tui {
  pub fn new(args: &Args, rg_results: VecDeque<RgMessageType>) -> Tui {
    Tui {
      app: App::new(args, rg_results),
    }
  }

  pub fn start(mut self) -> Result<()> {
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

      if self.app.should_quit {
        return Ok(());
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
