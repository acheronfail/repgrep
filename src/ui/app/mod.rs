mod app_events;
mod app_render;
mod state;

use std::collections::VecDeque;

use crate::model::{Item, PrintableStyle};
use crate::rg::de::{RgMessage, Stats};
pub use state::AppState;
use state::{AppListState, AppUiState};

pub struct App {
    pub state: AppState,

    rg_cmdline: String,
    stats: Stats,
    list: Vec<Item>,
    list_state: AppListState,
    ui_state: AppUiState,

    printable_style: PrintableStyle,
}

impl App {
    pub fn new(rg_cmdline: String, mut rg_results: VecDeque<RgMessage>) -> App {
        let mut list = vec![];
        let mut maybe_stats = None;
        while let Some(rg_message) = rg_results.pop_front() {
            match rg_message {
                RgMessage::Summary { stats, .. } => {
                    maybe_stats = Some(stats);
                    // NOTE: there should only be one RgMessage::Summary, and it should be the last item.
                    break;
                }
                other => list.push(Item::new(other)),
            }
        }

        App {
            state: AppState::Running,

            rg_cmdline,
            stats: maybe_stats.expect("failed to find RgMessage::Summary from rg!"),
            list_state: AppListState::new(),
            list,
            ui_state: AppUiState::SelectMatches,
            printable_style: PrintableStyle::Common,
        }
    }
}
