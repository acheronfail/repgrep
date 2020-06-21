use tui::style::{Color, Style};
use tui::widgets::Text;

/// Describes the various states that `App` can be in.
#[derive(Debug, Eq, PartialEq)]
pub enum AppState {
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
