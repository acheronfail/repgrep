use crate::model::PrintableStyle;
use crate::ui::app::AppListState;

/// Used when building the UI from the App's state.
pub struct UiItemContext<'a> {
    /// The replacement text the user has entered.
    pub replacement_text: Option<&'a str>,
    /// The current state of the matches list.
    pub ui_list_state: &'a AppListState,
    /// The `PrintableStyle` with which the UI should be built.
    pub printable_style: PrintableStyle,
}
