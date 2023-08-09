use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::ListState;

#[derive(Debug)]
pub struct AppListState {
    /// The selected "item" in the list of items received from rg
    selected_item: usize,
    /// The selected submatch of the selected "item"
    selected_submatch: usize,
    /// The position of the indicator on the left of the main list view
    indicator: ListState,
    /// The position of the start of the visible window in the main list view.
    /// We only send the visible lines to the renderer for performance reasons, and
    /// this represents the beginning of the visible window.
    window_start: usize,
}

impl AppListState {
    pub fn new() -> AppListState {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        AppListState {
            selected_item: 0,
            selected_submatch: 0,
            indicator: list_state,
            window_start: 0,
        }
    }

    pub fn indicator_mut(&mut self) -> &mut ListState {
        &mut self.indicator
    }

    pub fn set_indicator_pos(&mut self, idx: usize) {
        self.indicator.select(Some(idx));
    }

    pub fn window_start(&self) -> usize {
        self.window_start
    }

    pub fn set_window_start(&mut self, start: usize) {
        self.window_start = start;
    }

    pub fn selected_item(&self) -> usize {
        self.selected_item
    }

    pub fn selected_submatch(&self) -> usize {
        self.selected_submatch
    }

    pub fn set_selected_item(&mut self, idx: usize) {
        self.selected_item = idx
    }

    pub fn set_selected_submatch(&mut self, idx: usize) {
        self.selected_submatch = idx
    }
}

#[derive(Debug)]
pub enum AppState {
    Running,
    Cancelled,
    Complete,
}

/// Describes the various states that `App` can be in.
#[derive(Debug, Eq, PartialEq)]
pub enum AppUiState {
    /// Show the help text and keybindings.
    Help,
    /// The main matches list: select or deselect the found matches.
    SelectMatches,
    /// Prompt the user for the replacement text.
    /// (ReplacementText, CharPosition)
    InputReplacement(String, usize),
    /// Ask the user to confirm the replacement.
    /// (ReplacementText, CharPosition)
    ConfirmReplacement(String, usize),
}

impl AppUiState {
    pub fn is_replacing(&self) -> bool {
        matches!(
            self,
            AppUiState::InputReplacement(_, _) | AppUiState::ConfirmReplacement(_, _)
        )
    }

    pub fn user_replacement_text(&self) -> Option<&str> {
        match &self {
            AppUiState::InputReplacement(replacement, _)
            | AppUiState::ConfirmReplacement(replacement, _) => Some(replacement.as_str()),
            _ => None,
        }
    }

    /// Represent the `AppUiState` as a `Text`.
    /// This is displayed as the "mode" in the stats line.
    pub fn to_span(&self) -> Span {
        let style = Style::default().fg(Color::Black);
        match self {
            AppUiState::Help => Span::styled(" HELP ", style.bg(Color::Green)),
            AppUiState::SelectMatches => Span::styled(" SELECT ", style.bg(Color::Cyan)),
            AppUiState::InputReplacement(_, _) => Span::styled(" REPLACE ", style.bg(Color::White)),
            AppUiState::ConfirmReplacement(_, _) => Span::styled(" CONFIRM ", style.bg(Color::Red)),
        }
    }
}

/// A small struct to manage scrolling the text in the help view.
#[derive(Debug)]
pub struct HelpTextState {
    pub pos: usize,
    pub max: usize,
    help_text: &'static str,
}

impl HelpTextState {
    pub fn new(help_text: &'static str) -> HelpTextState {
        HelpTextState {
            pos: 0,
            max: help_text.lines().count() - 1,
            help_text,
        }
    }

    pub fn incr(&mut self) {
        if self.pos < self.max {
            self.pos += 1;
        }
    }

    pub fn decr(&mut self) {
        self.pos = self.pos.saturating_sub(1);
    }

    pub fn text(&self, num_lines: usize) -> String {
        self.help_text
            .lines()
            .skip(self.pos)
            .take(num_lines)
            .collect::<Vec<_>>()
            .join("\n")
    }
}
