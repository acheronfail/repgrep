mod app_events;
mod app_render;
mod state;

use crate::model::{Item, PrintableStyle};
use crate::rg::de::{RgMessage, Stats};
use state::HelpTextState;
pub use state::{AppListState, AppState, AppUiState};

const HELP_TEXT: &str = include_str!("../../../doc/rgr.1.template");

pub struct App {
    pub state: AppState,

    rg_cmdline: String,
    stats: Stats,
    list: Vec<Item>,
    list_state: AppListState,
    ui_state: AppUiState,
    help_text_state: HelpTextState,

    printable_style: PrintableStyle,
}

impl App {
    pub fn new(rg_cmdline: String, rg_messages: Vec<RgMessage>) -> App {
        let mut list = vec![];
        let mut maybe_stats = None;

        for (i, rg_message) in rg_messages.into_iter().enumerate() {
            match rg_message {
                RgMessage::Summary { stats, .. } => {
                    maybe_stats = Some(stats);
                    // NOTE: there should only be one RgMessage::Summary, and it should be the last item.
                    break;
                }
                other => list.push(Item::new(i, other)),
            }
        }

        App {
            state: AppState::Running,

            rg_cmdline,
            stats: maybe_stats.expect("failed to find RgMessage::Summary from rg!"),
            list_state: AppListState::new(),
            list,
            ui_state: AppUiState::SelectMatches,
            help_text_state: HelpTextState::new(HELP_TEXT),
            printable_style: PrintableStyle::Hidden,
        }
    }
}
