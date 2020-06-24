use tui::style::{Color, Style};
use tui::widgets::Text;

use crate::model::ReplacementCriteria;

#[derive(Debug)]
pub enum AppState {
    Running,
    Cancelled,
    Complete(ReplacementCriteria),
}

/// Describes the various states that `App` can be in.
#[derive(Debug, Eq, PartialEq)]
pub enum AppUiState {
    /// Show the help text and keybindings.
    Help,
    /// The main matches list: select or deselect the found matches.
    SelectMatches,
    /// Prompt the user for the replacement text.
    InputReplacement(String),
    /// Ask the user to confirm the replacement.
    ConfirmReplacement(String),
}

impl AppUiState {
    /// Represent the `AppUiState` as a `Text`.
    /// This is displayed as the "mode" in the stats line.
    pub fn to_text(&self) -> Text {
        let style = Style::default().fg(Color::Black);
        match self {
            AppUiState::Help => Text::styled(" HELP ", style.bg(Color::Green)),
            AppUiState::SelectMatches => Text::styled(" SELECT ", style.bg(Color::Cyan)),
            AppUiState::InputReplacement(_) => Text::styled(" REPLACE ", style.bg(Color::White)),
            AppUiState::ConfirmReplacement(_) => Text::styled(" CONFIRM ", style.bg(Color::Red)),
        }
    }
}
