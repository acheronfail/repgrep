mod app_events;
mod app_render;
mod state;

use anyhow::{bail, Result};
use regex::bytes::Regex;
use state::HelpTextState;
pub use state::{AppListState, AppState, AppUiState};

use crate::model::{PrintableStyle, ReplacementCriteria};
use crate::rg::de::{RgMessage, Stats};
use crate::ui::line::Item;

const HELP_TEXT: &str = include_str!("../../../doc/rgr.1.template");

pub struct App {
    pub state: AppState,

    /// If the user passed a regular expression with a capturing group, then this will be set to
    /// indicate that we should use the capturing group when performing replacements.
    capture_pattern: Option<Regex>,

    /// Raw args passed to `ripgrep`.
    rg_cmdline: String,
    /// Stats from `ripgrep`'s JSON output
    stats: Stats,
    /// A list that represents all matches and holds each match's state.
    list: Vec<Item>,
    /// State for where the user is inside the list.
    list_state: AppListState,
    /// Current UI mode.
    ui_state: AppUiState,
    /// Holds state information used when rendering the help screen.
    help_text_state: HelpTextState,

    /// The current printable style used to render text.
    printable_style: PrintableStyle,
}

impl App {
    pub fn new(
        capture_pattern: Option<Regex>,
        rg_cmdline: String,
        rg_messages: Vec<RgMessage>,
    ) -> App {
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

            capture_pattern,
            rg_cmdline,
            stats: maybe_stats.expect("failed to find RgMessage::Summary from rg!"),
            list_state: AppListState::new(),
            list,
            ui_state: AppUiState::SelectMatches,
            help_text_state: HelpTextState::new(HELP_TEXT),
            printable_style: PrintableStyle::default(),
        }
    }

    /// Consume the app and return `ReplacementCriteria`. This will return an `Err` if the app wasn't
    /// in a state where the user had entered any replacement text.
    pub fn get_replacement_criteria(self) -> Result<ReplacementCriteria> {
        match self.ui_state {
            AppUiState::InputReplacement(user_replacement, _)
            | AppUiState::ConfirmReplacement(user_replacement, _) => Ok(ReplacementCriteria::new(
                self.capture_pattern,
                user_replacement,
                self.list,
            )),
            other => bail!(
                "unexpected app ui state when calling App::get_replacement_criteria: {:?}",
                other
            ),
        }
    }
}
