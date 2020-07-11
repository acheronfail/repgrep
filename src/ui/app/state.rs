use tui::style::{Color, StyleDiff};
use tui::text::Span;
use tui::widgets::ListState;

use crate::model::ReplacementCriteria;

#[derive(Debug)]
pub struct AppListState(ListState, usize);

impl AppListState {
    pub fn new() -> AppListState {
        let mut list_row_state = ListState::default();
        list_row_state.select(Some(0));
        AppListState(list_row_state, 0)
    }

    pub fn row_state_mut(&mut self) -> &mut ListState {
        &mut self.0
    }

    pub fn row_col(&self) -> (usize, usize) {
        (self.row(), self.col())
    }

    pub fn row(&self) -> usize {
        self.0.selected().unwrap_or(0)
    }

    pub fn col(&self) -> usize {
        self.1
    }

    pub fn set_row_col(&mut self, row: usize, col: usize) {
        self.set_row(row);
        self.set_col(col);
    }

    pub fn set_row(&mut self, row: usize) {
        self.0.select(Some(row));
    }

    pub fn set_col(&mut self, col: usize) {
        self.1 = col;
    }
}

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
    pub fn to_span(&self) -> Span {
        let style = StyleDiff::default().fg(Color::Black);
        match self {
            AppUiState::Help => Span::styled(" HELP ", style.bg(Color::Green)),
            AppUiState::SelectMatches => Span::styled(" SELECT ", style.bg(Color::Cyan)),
            AppUiState::InputReplacement(_) => Span::styled(" REPLACE ", style.bg(Color::White)),
            AppUiState::ConfirmReplacement(_) => Span::styled(" CONFIRM ", style.bg(Color::Red)),
        }
    }
}
