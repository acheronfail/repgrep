use ratatui::layout::Rect;

use crate::model::{CaptureMatcher, PrintableStyle};
use crate::ui::app::{AppListState, AppUiState};

/// Used when building the UI from the App's state.
pub struct UiItemContext<'a> {
    /// Regex to use for capture expansion. If it isn't provided, only capture
    /// group 0 is available from the matched text.
    pub capture_pattern: Option<&'a CaptureMatcher>,
    /// The replacement text the user has entered.
    pub replacement_text: Option<&'a str>,
    /// The current state of the matches list.
    pub app_list_state: &'a AppListState,
    /// The current UI state of the App.
    pub app_ui_state: &'a AppUiState,
    /// The `PrintableStyle` with which the UI should be built.
    pub printable_style: PrintableStyle,
    /// The `Rect` that the items will be rendered into.
    pub list_rect: Rect,
}
