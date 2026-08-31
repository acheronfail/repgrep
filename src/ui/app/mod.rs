mod app_events;
mod app_render;
mod state;

use anyhow::{Result, bail};
use state::HelpTextState;
pub use state::{AppListState, AppState, AppUiState};

use crate::model::{CaptureMatcher, PrintableStyle, ReplacementCriteria, replacement_items};
use crate::rg::de::{RgMessage, Stats};
use crate::ui::line::Item;
use crate::ui::theme::Theme;

const HELP_TEXT: &str = include_str!("../../../doc/rgr.1.template");

pub struct App {
    pub state: AppState,

    /// Replacement text carried between the match-selection and replacement-input modes.
    replacement_draft: String,

    /// If the user passed a single regular expression, then this will be set so capture groups can
    /// be expanded when performing replacements. Capture group 0 contains the full match.
    capture_pattern: Option<CaptureMatcher>,

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
    /// A count prefix typed by the user (e.g. "2" for 2j).
    count: Option<u32>,
    /// Holds state information used when rendering the help screen.
    help_text_state: HelpTextState,

    /// The current printable style used to render text.
    printable_style: PrintableStyle,
    /// Styles used to render the UI.
    theme: Theme,
}

impl App {
    pub fn new(
        capture_pattern: Option<CaptureMatcher>,
        rg_cmdline: String,
        rg_messages: Vec<RgMessage>,
        replacement: Option<String>,
    ) -> App {
        let (list, maybe_stats) = replacement_items(rg_messages);

        App {
            state: AppState::Running,

            replacement_draft: replacement.unwrap_or_default(),

            capture_pattern,
            rg_cmdline,
            stats: maybe_stats.expect("failed to find RgMessage::Summary from rg!"),
            list_state: AppListState::new(),
            list,
            ui_state: AppUiState::SelectMatches,
            count: None,
            help_text_state: HelpTextState::new(HELP_TEXT),
            printable_style: PrintableStyle::default(),
            theme: Theme::default(),
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
