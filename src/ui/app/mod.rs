mod application;
mod item;

use tui::style::{Color, Style};
use tui::widgets::Text;

pub use application::App;
pub use item::{Item, ItemKind};

/// Defines basic movement types in the main matches list.
#[derive(Debug, Eq, PartialEq)]
enum Movement {
  /// Move to the previous item.
  Prev,
  /// Move to the next item.
  Next,
  /// Move to the previous file.
  PrevFile,
  /// Move to the next file.
  NextFile,
  /// Move forward `n` items.
  Forward(u16),
  /// Move backward `n` items.
  Backward(u16),
}

/// Describes the various states that `App` can be in.
#[derive(Debug, Eq, PartialEq)]
enum AppState {
  /// Show the help text and keybindings.
  Help,
  /// The main matches list: select or deselect the found matches.
  SelectMatches,
  /// Prompt the user for the replacement text.
  InputReplacement(String),
  /// Ask the user to confirm the replacement.
  ConfirmReplacement(String),
}

impl AppState {
  /// Represent the `AppState` as a `Text`.
  /// This is displayed as the "mode" in the stats line.
  pub fn to_text(&self) -> Text {
    let style = Style::default().fg(Color::Black);
    match self {
      AppState::Help => Text::styled(" HELP ", style.bg(Color::Green)),
      AppState::SelectMatches => Text::styled(" SELECT ", style.bg(Color::Cyan)),
      AppState::InputReplacement(_) => Text::styled(" REPLACE ", style.bg(Color::White)),
      AppState::ConfirmReplacement(_) => Text::styled(" CONFIRM ", style.bg(Color::Red)),
    }
  }
}
