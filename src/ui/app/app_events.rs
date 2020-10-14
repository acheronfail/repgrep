/// Event handling for `App`.
use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use either::Either;
use tui::layout::Rect;

use crate::model::{Movement, ReplacementCriteria};
use crate::rg::de::RgMessageKind;
use crate::ui::app::{App, AppState, AppUiState};
use crate::util::clamp;

impl App {
    pub fn on_event(&mut self, term_size: Rect, event: Event) -> Result<()> {
        if let Event::Key(key) = event {
            // Common Ctrl+Key scroll keybindings that apply to multiple modes.
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                let did_handle_key = match &self.ui_state {
                    AppUiState::SelectMatches
                    | AppUiState::InputReplacement(_)
                    | AppUiState::ConfirmReplacement(_) => match key.code {
                        // Page movements
                        KeyCode::Char('b') => {
                            self.move_pos(Movement::Backward(self.list_height(term_size)));
                            true
                        }
                        KeyCode::Char('f') => {
                            self.move_pos(Movement::Forward(self.list_height(term_size)));
                            true
                        }

                        // Toggle whitespace style
                        KeyCode::Char('v') => {
                            self.printable_style = self.printable_style.swap();
                            true
                        }
                        _ => false,
                    },
                    _ => false,
                };

                // If a key was handled then stop processing any other events.
                if did_handle_key {
                    return Ok(());
                }
            }

            match &self.ui_state {
                AppUiState::ConfirmReplacement(replacement) => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.ui_state = AppUiState::InputReplacement(replacement.to_owned())
                    }
                    KeyCode::Enter => {
                        self.state = AppState::Complete(ReplacementCriteria::new(
                            replacement,
                            self.list.clone(),
                        ));
                    }
                    _ => {}
                },
                AppUiState::Help => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => self.ui_state = AppUiState::SelectMatches,
                    KeyCode::Char('k') | KeyCode::Up => self.help_text_state.decr(),
                    KeyCode::Char('j') | KeyCode::Down => self.help_text_state.incr(),
                    _ => {}
                },
                AppUiState::SelectMatches => {
                    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                            self.move_pos(if shift {
                                Movement::PrevFile
                            } else {
                                Movement::PrevLine
                            })
                        }
                        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                            self.move_pos(if shift {
                                Movement::NextFile
                            } else {
                                Movement::NextLine
                            })
                        }
                        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                            self.move_pos(Movement::Prev)
                        }
                        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                            self.move_pos(Movement::Next)
                        }
                        KeyCode::Char(' ') => self.toggle_item(false),
                        KeyCode::Char('s') | KeyCode::Char('S') => self.toggle_item(true),
                        KeyCode::Char('a') | KeyCode::Char('A') => self.toggle_all_items(),
                        KeyCode::Esc | KeyCode::Char('q') => self.state = AppState::Cancelled,
                        KeyCode::Char('?') => self.ui_state = AppUiState::Help,
                        KeyCode::Enter | KeyCode::Char('r') | KeyCode::Char('R') => {
                            self.ui_state = AppUiState::InputReplacement(String::new())
                        }
                        _ => {}
                    }
                }
                AppUiState::InputReplacement(ref input) => match key.code {
                    KeyCode::Char(c) => {
                        let mut new_input = String::from(input);
                        new_input.push(c);
                        self.ui_state = AppUiState::InputReplacement(new_input);
                    }
                    KeyCode::Backspace | KeyCode::Delete => {
                        let new_input = if !input.is_empty() {
                            String::from(input)[..input.len() - 1].to_owned()
                        } else {
                            String::new()
                        };
                        self.ui_state = AppUiState::InputReplacement(new_input);
                    }
                    KeyCode::Esc => self.ui_state = AppUiState::SelectMatches,
                    KeyCode::Enter => {
                        self.ui_state = AppUiState::ConfirmReplacement(input.to_owned())
                    }
                    _ => {}
                },
            }
        }

        Ok(())
    }

    fn move_horizonally(&mut self, movement: &Movement) -> bool {
        let selected_item = self.list_state.selected_item();
        let selected_match = self.list_state.selected_submatch();

        // Handle moving horizontally.
        if matches!(movement, Movement::Next) && selected_match + 1 < self.list[selected_item].sub_items().len() {
            self.list_state.set_selected_submatch(selected_match + 1);
            return true;
        } else if matches!(movement, Movement::Prev) && selected_match > 0 {
            self.list_state.set_selected_submatch(selected_match - 1);
            return true;
        }

        false
    }

    fn move_vertically(&mut self, movement: &Movement) {
        // Reverse the iterator depending on movement direction.
        let iterator = {
            let iter = self.list.iter().enumerate();
            if movement.is_forward() {
                Either::Right(iter)
            } else {
                Either::Left(iter.rev())
            }
        };

        // Determine how far to skip down the list.
        let selected_item = self.list_state.selected_item();
        let (skip, default_item_idx) = match movement {
            Movement::Prev | Movement::PrevLine | Movement::PrevFile => {
                (self.list.len().saturating_sub(selected_item), 0)
            }
            Movement::Backward(n) => (
                self.list
                    .len()
                    .saturating_sub(selected_item.saturating_sub(*n as usize)),
                0,
            ),

            Movement::Next | Movement::NextLine | Movement::NextFile => (selected_item, self.list.len() - 1),
            Movement::Forward(n) => (selected_item + (*n as usize), self.list.len() - 1),
        };

        // Find the new position.
        let (item_idx, match_idx) = iterator
            .skip(skip)
            .find_map(|(i, item)| {
                let is_valid_next = match movement {
                    Movement::PrevFile => i < selected_item && matches!(item.kind, RgMessageKind::Begin),
                    Movement::NextFile => i > selected_item && matches!(item.kind, RgMessageKind::Begin),
                    Movement::Prev | Movement::PrevLine | Movement::Backward(_) => i < selected_item,
                    Movement::Next | Movement::NextLine | Movement::Forward(_) => i > selected_item,
                };

                if is_valid_next && item.is_selectable() {
                    if matches!(movement, Movement::Prev) {
                        Some((i, item.sub_items().len().saturating_sub(1)))
                    } else {
                        Some((i, 0))
                    }
                } else {
                    None
                }
            })
            .unwrap_or((default_item_idx, 0));

        let item_idx = clamp(item_idx, 0, self.list.len() - 1);
        self.list_state.set_selected_item(item_idx);
        self.list_state.set_selected_submatch(match_idx);
        // TODO: set this accordingly when multiline is supported
        self.list_state.set_indicator(item_idx);
    }

    pub(crate) fn move_pos(&mut self, movement: Movement) {
        if self.move_horizonally(&movement) {
            return;
        }

        self.move_vertically(&movement);
    }

    pub(crate) fn toggle_item(&mut self, all_sub_items: bool) {
        let selected_item = self.list_state.selected_item();
        let selected_match = self.list_state.selected_submatch();

        // If Match item, toggle replace.
        if matches!(self.list[selected_item].kind, RgMessageKind::Match) {
            let selected_item = &mut self.list[selected_item];
            if all_sub_items {
                let should_replace = !selected_item.get_should_replace_all();
                selected_item.set_should_replace_all(should_replace);
            } else {
                selected_item.set_should_replace(selected_match, !selected_item.get_should_replace(selected_match));
            }
        }

        // If Begin item, toggle all matches in it.
        if matches!(self.list[selected_item].kind, RgMessageKind::Begin) {
            let mut items_to_toggle: Vec<_> = self
                .list
                .iter_mut()
                .skip(selected_item)
                .take_while(|i| i.kind != RgMessageKind::End)
                .filter(|i| i.kind == RgMessageKind::Match)
                .collect();

            let should_replace = items_to_toggle.iter().all(|i| !i.get_should_replace_all());
            for item in items_to_toggle.iter_mut() {
                item.set_should_replace_all(should_replace);
            }
        }
    }

    pub(crate) fn toggle_all_items(&mut self) {
        let should_replace = !self.list.iter().all(|i| i.get_should_replace_all());

        for item in self.list.iter_mut() {
            item.set_should_replace_all(should_replace);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use pretty_assertions::assert_eq;

    use crate::rg::de::test_utilities::*;
    use crate::rg::de::*;
    use crate::ui::app::*;

    fn rg_messages() -> Vec<RgMessage> {
        vec![
            RgMessage::from_str(RG_JSON_BEGIN),
            RgMessage::from_str(RG_JSON_MATCH),
            RgMessage::from_str(RG_JSON_CONTEXT),
            RgMessage::from_str(RG_JSON_MATCH),
            RgMessage::from_str(RG_JSON_CONTEXT),
            RgMessage::from_str(RG_JSON_END),
            RgMessage::from_str(RG_JSON_SUMMARY),
        ]
    }

    fn items() -> Vec<Item> {
        let mut messages = rg_messages();
        messages
            .drain(..messages.len() - 1)
            .map(|m| Item::new(m))
            .collect()
    }

    fn new_app() -> App {
        App::new("TESTS".to_string(), VecDeque::from(rg_messages()))
    }

    #[test]
    fn it_toggles_item_all_sub_items() {
        let mut app = new_app();
        let mut expected_items = items();
        assert_eq!(app.list, expected_items);

        // Toggle all sub items
        app.list_state.set_selected_item(1);
        app.list_state.set_selected_submatch(0);
        app.toggle_item(true);

        // Should have only toggled that one.
        expected_items[1].set_should_replace(0, false);
        expected_items[1].set_should_replace(1, false);
        assert_eq!(app.list, expected_items);
    }

    #[test]
    fn it_toggles_item_sub_item() {
        let mut app = new_app();
        let mut expected_items = items();
        assert_eq!(app.list, expected_items);

        // Toggle a single sub item
        app.list_state.set_selected_item(1);
        app.list_state.set_selected_submatch(0);
        app.toggle_item(false);

        // Should have only toggled that one.
        expected_items[1].set_should_replace(0, false);
        assert_eq!(app.list, expected_items);
    }

    #[test]
    fn it_toggles_all_items() {
        let mut app = new_app();
        let mut expected_items = items();
        assert_eq!(app.list, expected_items);

        // Toggle them all off
        app.toggle_all_items();
        expected_items[1].set_should_replace_all(false);
        expected_items[3].set_should_replace_all(false);
        assert_eq!(app.list, expected_items);

        // Toggle them all back on
        app.toggle_all_items();
        expected_items[1].set_should_replace_all(true);
        expected_items[3].set_should_replace_all(true);
        assert_eq!(app.list, expected_items);
    }

    #[test]
    fn it_toggles_all_items_with_item_off() {
        let mut app = new_app();
        let expected_items = items();
        assert_eq!(app.list, expected_items);

        // Turn off a single item
        app.list[1].set_should_replace(0, false);
        app.list[1].set_should_replace(1, false);
        // Now toggle all, they should all be on
        app.toggle_all_items();
        assert_eq!(app.list, expected_items);
    }

    #[test]
    fn it_toggles_all_items_with_sub_item_off() {
        let mut app = new_app();
        let expected_items = items();
        assert_eq!(app.list, expected_items);

        // Turn off a single sub item
        app.list[1].set_should_replace(1, false);
        // Now toggle all, they should all be on
        app.toggle_all_items();
        assert_eq!(app.list, expected_items);
    }
}
