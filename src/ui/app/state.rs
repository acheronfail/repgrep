use tui::style::{Color, StyleDiff};
use tui::text::{Span, Spans};

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
    pub fn to_text(&self) -> Spans {
        let style = StyleDiff::default().fg(Color::Black);
        match self {
            AppUiState::Help => Spans::from(Span::styled(" HELP ", style.bg(Color::Green))),
            AppUiState::SelectMatches => {
                Spans::from(Span::styled(" SELECT ", style.bg(Color::Cyan)))
            }
            AppUiState::InputReplacement(_) => {
                Spans::from(Span::styled(" REPLACE ", style.bg(Color::White)))
            }
            AppUiState::ConfirmReplacement(_) => {
                Spans::from(Span::styled(" CONFIRM ", style.bg(Color::Red)))
            }
        }
    }
}
