use tui::widgets::ListItem;

use crate::model::PrintableStyle;
use crate::ui::app::AppListState;

pub struct UiItemContext<'a> {
    pub replacement_text: Option<&'a str>,
    pub ui_list_state: AppListState,
    pub printable_style: PrintableStyle,
}

pub trait ToListItem {
    fn to_list_item(&self, ctx: &UiItemContext) -> Vec<ListItem>;
}
