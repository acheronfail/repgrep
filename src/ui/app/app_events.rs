/// Event handling for `App`.
use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use either::Either;
use ratatui::layout::Rect;

use crate::model::Movement;
use crate::rg::de::RgMessageKind;
use crate::ui::app::{App, AppState, AppUiState};
use crate::util::{byte_pos_from_char_pos, clamp};

impl App {
    pub fn on_event(&mut self, term_size: Rect, event: Event) -> Result<()> {
        match event {
            Event::Resize(w, h) => {
                let new_size = Rect::new(term_size.x, term_size.y, w, h);
                self.update_indicator(new_size);
            }
            Event::Key(key) if self.is_frame_too_small(term_size) => {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => self.state = AppState::Cancelled,
                    _ => {}
                }
                return Ok(());
            }
            Event::Key(key) => {
                // We only care about `Press` events. Other events such as `Release` and `Repeat` aren't
                // fired on every terminal, and we don't need them anyway.
                if !matches!(key.kind, KeyEventKind::Press) {
                    return Ok(());
                }

                let control_pressed = key.modifiers.contains(KeyModifiers::CONTROL);
                if control_pressed {
                    // Clear input on Ctrl+U
                    if let AppUiState::InputReplacement(_, _) = &self.ui_state {
                        if key.code == KeyCode::Char('u') {
                            self.ui_state = AppUiState::InputReplacement(String::new(), 0);
                            return Ok(());
                        }
                    }

                    // Common Ctrl+Key scroll keybindings that apply to multiple modes.
                    if matches!(
                        &self.ui_state,
                        AppUiState::SelectMatches
                            | AppUiState::InputReplacement(_, _)
                            | AppUiState::ConfirmReplacement(_, _)
                    ) {
                        match key.code {
                            // Page movements
                            KeyCode::Char('b') => {
                                self.move_pos(
                                    Movement::Backward(self.main_view_list_rect(term_size).height),
                                    term_size,
                                );
                                return Ok(());
                            }
                            KeyCode::Char('f') => {
                                self.move_pos(
                                    Movement::Forward(self.main_view_list_rect(term_size).height),
                                    term_size,
                                );
                                return Ok(());
                            }

                            // Toggle whitespace style
                            KeyCode::Char('v') => {
                                self.printable_style = self.printable_style.cycle();
                                self.update_indicator(term_size);
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                }

                match &self.ui_state {
                    AppUiState::ConfirmReplacement(replacement, pos) => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            self.ui_state =
                                AppUiState::InputReplacement(replacement.to_owned(), *pos)
                        }
                        KeyCode::Enter => {
                            self.state = AppState::Complete;
                            return Ok(());
                        }
                        _ => {}
                    },
                    AppUiState::Help => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            self.ui_state = AppUiState::SelectMatches
                        }
                        KeyCode::Char('k') | KeyCode::Up => self.help_text_state.decr(),
                        KeyCode::Char('j') | KeyCode::Down => self.help_text_state.incr(),
                        _ => {}
                    },
                    AppUiState::SelectMatches => {
                        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                        match key.code {
                            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => self.move_pos(
                                if shift {
                                    Movement::PrevFile
                                } else {
                                    Movement::PrevLine
                                },
                                term_size,
                            ),
                            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => self
                                .move_pos(
                                    if shift {
                                        Movement::NextFile
                                    } else {
                                        Movement::NextLine
                                    },
                                    term_size,
                                ),
                            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                                self.move_pos(Movement::Prev, term_size)
                            }
                            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                                self.move_pos(Movement::Next, term_size)
                            }
                            KeyCode::Char(' ') => self.toggle_item(false),
                            KeyCode::Char('s') | KeyCode::Char('S') => self.toggle_item(true),
                            KeyCode::Char('a') | KeyCode::Char('A') => self.toggle_all_items(),
                            KeyCode::Char('v') => self.invert_selection_current(),
                            KeyCode::Char('V') => self.invert_selection_all(),
                            KeyCode::Esc | KeyCode::Char('q') => self.state = AppState::Cancelled,
                            KeyCode::Char('?') => self.ui_state = AppUiState::Help,
                            KeyCode::Enter | KeyCode::Char('r') | KeyCode::Char('R') => {
                                self.ui_state = AppUiState::InputReplacement(String::new(), 0)
                            }
                            _ => {}
                        }
                    }
                    AppUiState::InputReplacement(ref input, pos) => match key.code {
                        // input char, or detect changing to next mode
                        KeyCode::Char(ch) => {
                            if control_pressed && ch == 's' {
                                self.ui_state =
                                    AppUiState::ConfirmReplacement(input.to_owned(), *pos);
                            } else {
                                let mut new_input = input.clone();
                                new_input.insert(byte_pos_from_char_pos(input, *pos), ch);
                                self.ui_state = AppUiState::InputReplacement(new_input, pos + 1);
                            }
                        }
                        // remove character behind cursor
                        KeyCode::Backspace => {
                            if !input.is_empty() && *pos > 0 {
                                let mut new_input = input.clone();
                                new_input.remove(byte_pos_from_char_pos(input, *pos - 1));
                                self.ui_state = AppUiState::InputReplacement(new_input, pos - 1);
                            }
                        }
                        // remove character at cursor
                        KeyCode::Delete => {
                            if !input.is_empty() && *pos < input.chars().count() {
                                let mut new_input = input.clone();
                                new_input.remove(byte_pos_from_char_pos(input, *pos));
                                self.ui_state = AppUiState::InputReplacement(new_input, *pos);
                            }
                        }
                        // leave mode
                        KeyCode::Esc => self.ui_state = AppUiState::SelectMatches,
                        // insert return character
                        KeyCode::Enter => {
                            let mut new_input = input.clone();
                            new_input.insert(byte_pos_from_char_pos(input, *pos), '\n');
                            self.ui_state = AppUiState::InputReplacement(new_input, pos + 1);
                        }
                        // move cursor back
                        KeyCode::Left => {
                            self.ui_state =
                                AppUiState::InputReplacement(input.clone(), pos.saturating_sub(1))
                        }
                        // move cursor forward
                        KeyCode::Right => {
                            self.ui_state = AppUiState::InputReplacement(
                                input.clone(),
                                (pos + 1).clamp(0, input.chars().count()),
                            )
                        }
                        // move to start
                        KeyCode::Home | KeyCode::PageUp => {
                            self.ui_state = AppUiState::InputReplacement(input.clone(), 0)
                        }
                        // move to end
                        KeyCode::End | KeyCode::PageDown => {
                            self.ui_state =
                                AppUiState::InputReplacement(input.clone(), input.chars().count())
                        }
                        _ => {}
                    },
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn move_horizontally(&mut self, movement: &Movement) -> bool {
        let selected_item = self.list_state.selected_item();
        let selected_match = self.list_state.selected_submatch();

        // Handle moving horizontally.
        if matches!(movement, Movement::Next)
            && selected_match + 1 < self.list[selected_item].sub_items().len()
        {
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
                    .saturating_sub(selected_item.saturating_sub((*n - 1) as usize)),
                0,
            ),

            Movement::Next | Movement::NextLine | Movement::NextFile => {
                (selected_item, self.list.len() - 1)
            }
            Movement::Forward(n) => (selected_item + (*n as usize), self.list.len() - 1),
        };

        // Find the new position.
        let (item_idx, match_idx) = iterator
            .skip(skip)
            .find_map(|(i, item)| {
                let is_valid_next = match movement {
                    Movement::PrevFile => {
                        i < selected_item && matches!(item.kind, RgMessageKind::Begin)
                    }
                    Movement::NextFile => {
                        i > selected_item && matches!(item.kind, RgMessageKind::Begin)
                    }
                    Movement::Prev | Movement::PrevLine | Movement::Backward(_) => {
                        i < selected_item
                    }
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
    }

    /// Update the UI's indicator position to point to the start of the selected item, and in the case of
    /// a match which spans multiple lines and has multiple submatches, the start of the selected submatch.
    /// Note that this is also the mechanism which scrolls ratatui's list interface.
    fn update_indicator(&mut self, term_size: Rect) {
        let item_idx = self.list_state.selected_item();
        let match_idx = self.list_state.selected_submatch();
        let main_view_list_rect = self.main_view_list_rect(term_size);

        let mut indicator_idx = 0;
        for item in &mut self.list.as_mut_slice()[0..item_idx] {
            let item_height = item.line_count(main_view_list_rect.width, self.printable_style);
            indicator_idx += item_height;
        }

        let height_to_sub_item = self.list[item_idx]
            .line_count_at(match_idx, main_view_list_rect.width, self.printable_style)
            // sub 1 here because the indicator starts at position 1 of the item
            .saturating_sub(1);
        indicator_idx += height_to_sub_item;

        // update visible window region is required
        {
            let height = main_view_list_rect.height as usize;
            let mut window_start = self.list_state.window_start();
            // scrolling down past bottom of viewport
            if indicator_idx >= window_start + height {
                window_start = indicator_idx - height + 1;
            }
            // scrolling up past top of viewport
            if indicator_idx < window_start {
                window_start = indicator_idx;
            }
            self.list_state.set_window_start(window_start);
        }

        // adjust `indicator_idx` by the start of the window, since we only pass
        // the visible lines to the terminal (but our indices are absolute)
        self.list_state
            .set_indicator_pos(indicator_idx - self.list_state.window_start());
    }

    pub(crate) fn move_pos(&mut self, movement: Movement, term_size: Rect) {
        if !self.move_horizontally(&movement) {
            self.move_vertically(&movement);
        }

        self.update_indicator(term_size);
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
                selected_item.set_should_replace(
                    selected_match,
                    !selected_item.get_should_replace(selected_match),
                );
            }
        }

        // If Begin item, toggle all matches in it.
        if matches!(self.list[selected_item].kind, RgMessageKind::Begin) {
            let mut items_to_toggle = self.get_all_items_in_file(selected_item);
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

    fn invert_selection_current(&mut self) {
        let selected_item = self.list_state.selected_item();

        match self.list[selected_item].kind {
            RgMessageKind::Match => self.list[selected_item].invert_selection(),
            RgMessageKind::Begin => {
                for item in self.get_all_items_in_file(selected_item) {
                    item.invert_selection();
                }
            }
            _ => {}
        }
    }

    fn get_all_items_in_file(&mut self, selected_item: usize) -> Vec<&mut crate::ui::line::Item> {
        self.list
            .iter_mut()
            .skip(selected_item)
            .take_while(|i| i.kind != RgMessageKind::End)
            .filter(|i| i.kind == RgMessageKind::Match)
            .collect()
    }

    fn invert_selection_all(&mut self) {
        for item in self.list.iter_mut() {
            item.invert_selection();
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use pretty_assertions::assert_eq;
    use ratatui::layout::Rect;

    use crate::model::Movement;
    use crate::rg::de::test_utilities::*;
    use crate::rg::de::*;
    use crate::ui::app::*;

    impl App {
        fn current_item(&mut self) -> &mut Item {
            &mut self.list[self.list_state.selected_item()]
        }
    }

    fn app_list_to_match_replace(app: &App) -> Vec<bool> {
        app.list
            .iter()
            .filter(|i| matches!(i.kind, RgMessageKind::Match))
            .map(|i| i.get_should_replace_all())
            .collect::<Vec<bool>>()
    }

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
            .enumerate()
            .map(|(i, m)| Item::new(i, m))
            .collect()
    }

    fn new_app() -> App {
        App::new(None, "TESTS".to_string(), rg_messages())
    }

    fn new_app_multiple_files() -> App {
        let mut messages_multiple_files = vec![];

        let messages = rg_messages();
        messages_multiple_files.extend_from_slice(&messages[0..messages.len() - 1]);
        messages_multiple_files.extend(vec![
            RgMessage::from_str(RG_JSON_BEGIN),
            RgMessage::from_str(RG_JSON_MATCH_MULTILINE),
            RgMessage::from_str(RG_JSON_END),
        ]);
        messages_multiple_files.extend(messages_multiple_files.clone());
        messages_multiple_files.push(RgMessage::from_str(RG_JSON_SUMMARY));

        App::new(None, "TESTS".to_string(), messages_multiple_files)
    }

    type PosTriple = (usize, usize, usize);

    // Valid positions for the app returned by `new_app_multiple_files`.
    const POS_1_BEGIN: PosTriple = (0, 0, 0);
    const POS_1_MATCH_0_0: PosTriple = (1, 0, 1);
    const POS_1_MATCH_0_1: PosTriple = (1, 1, 1);
    const POS_1_MATCH_1_0: PosTriple = (3, 0, 3);
    const POS_1_MATCH_1_1: PosTriple = (3, 1, 3);
    const POS_2_BEGIN: PosTriple = (6, 0, 6);
    const POS_2_MATCH_MULTILINE_0_0: PosTriple = (7, 0, 7);
    const POS_2_MATCH_MULTILINE_0_1: PosTriple = (7, 1, 9);
    const POS_3_BEGIN: PosTriple = (9, 0, 11);
    const POS_3_MATCH_0_0: PosTriple = (10, 0, 12);
    const POS_3_MATCH_0_1: PosTriple = (10, 1, 12);
    const POS_3_MATCH_1_0: PosTriple = (12, 0, 14);
    const POS_3_MATCH_1_1: PosTriple = (12, 1, 14);
    const POS_4_BEGIN: PosTriple = (15, 0, 17);
    const POS_4_MATCH_MULTILINE_0_0: PosTriple = (16, 0, 18);
    const POS_4_MATCH_MULTILINE_0_1: PosTriple = (16, 1, 20);
    const POS_4_END: PosTriple = (17, 0, 21);

    fn new_app_line_wrapping() -> App {
        let messages = vec![
            RgMessage::from_str(RG_JSON_BEGIN),
            RgMessage::from_str(RG_JSON_CONTEXT_LINE_WRAP),
            RgMessage::from_str(RG_JSON_MATCH_LINE_WRAP),
            RgMessage::from_str(RG_JSON_MATCH_LINE_WRAP_MULTI),
            RgMessage::from_str(RG_JSON_END),
            RgMessage::from_str(RG_JSON_SUMMARY),
        ];

        App::new(None, "TESTS".to_string(), messages)
    }

    // Valid positions for the app returned by `new_app_line_wrapping`.
    const POS_WRAP_BEGIN: PosTriple = (0, 0, 0);
    const POS_WRAP_MATCH: PosTriple = (2, 0, 4);
    const POS_WRAP_MATCH_MULTI_0_1: PosTriple = (3, 0, 5);
    const POS_WRAP_MATCH_MULTI_0_2: PosTriple = (3, 1, 5);
    const POS_WRAP_MATCH_MULTI_0_3: PosTriple = (3, 2, 5);
    const POS_WRAP_MATCH_MULTI_0_4: PosTriple = (3, 3, 6);
    const POS_WRAP_MATCH_MULTI_0_5: PosTriple = (3, 4, 6);
    const POS_WRAP_MATCH_MULTI_0_6: PosTriple = (3, 5, 6);
    const POS_WRAP_MATCH_MULTI_0_7: PosTriple = (3, 6, 7);
    const POS_WRAP_END: PosTriple = (4, 0, 8);

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

    #[test]
    fn it_inverts_a_match() {
        let mut app = new_app();
        let expected_items = items();
        assert_eq!(app.list, expected_items);

        // invert selection on match
        app.list[1].invert_selection();
        assert_eq!(app.list[1].get_should_replace_all(), false);
        app.list[1].invert_selection();
        assert_eq!(app.list[1].get_should_replace_all(), true);
        assert_eq!(app.list, expected_items);
    }

    // Inverting selection

    #[test]
    fn it_inverts_a_file() {
        let mut app = new_app_multiple_files();
        let term_size = Rect::new(0, 0, 80, 24);
        let expected_items = app.list.clone();

        // sanity check
        assert_eq!(app.list, expected_items);
        assert_eq!(app.current_item().kind, RgMessageKind::Begin);

        // select match and invert
        app.move_pos(Movement::NextLine, term_size);
        assert_eq!(app.current_item().kind, RgMessageKind::Match);
        app.invert_selection_current();

        // select file containing match
        app.move_pos(Movement::PrevFile, term_size);
        assert_eq!(app.current_item().kind, RgMessageKind::Begin);

        // NOTE: last four matches are in different files, so stay the same
        assert_eq!(
            app_list_to_match_replace(&app),
            vec![false, true, true, true, true, true]
        );

        // invert file
        app.invert_selection_current();
        assert_eq!(
            app_list_to_match_replace(&app),
            vec![true, false, true, true, true, true]
        );
    }

    #[test]
    fn it_inverts_all() {
        let mut app = new_app_multiple_files();
        let term_size = Rect::new(0, 0, 80, 24);
        let expected_items = app.list.clone();

        // sanity check
        assert_eq!(app.list, expected_items);
        assert_eq!(app.current_item().kind, RgMessageKind::Begin);

        // select match in first file and invert
        app.move_pos(Movement::NextLine, term_size);
        assert_eq!(app.current_item().kind, RgMessageKind::Match);
        app.invert_selection_current();

        // select next file and invert
        // NOTE: this file only has a single match
        app.move_pos(Movement::NextFile, term_size);
        assert_eq!(app.current_item().kind, RgMessageKind::Begin);
        app.invert_selection_current();

        // invert all
        let expected = vec![false, true, false, true, true, true];
        assert_eq!(app_list_to_match_replace(&app), expected);
        app.invert_selection_all();
        assert_eq!(
            app_list_to_match_replace(&app),
            expected.iter().map(|b| !b).collect::<Vec<_>>()
        );
    }

    // Movement

    fn get_indicator(list_state: &mut AppListState) -> usize {
        list_state.indicator_mut().selected().unwrap()
    }

    macro_rules! assert_list_state {
        ($app:expr, $triple:expr) => {
            let selected_item = $app.list_state.selected_item();
            let selected_submatch = $app.list_state.selected_submatch();
            let indicator = get_indicator(&mut $app.list_state);
            assert_eq!((selected_item, selected_submatch, indicator), $triple);
        };
    }

    macro_rules! move_and_assert_list_state {
        ($app:expr, $movement:expr, $triple:expr) => {
            move_and_assert_list_state!($app, $movement, $triple, Rect::new(0, 0, 80, 24))
        };
        ($app:expr, $movement:expr, $triple:expr, $rect:expr) => {
            $app.move_pos($movement, $rect);
            assert_list_state!($app, $triple);
        };
    }

    #[test]
    fn movement_line_wrapping() {
        let mut app = new_app_line_wrapping();
        assert_list_state!(app, POS_WRAP_BEGIN);
        move_and_assert_list_state!(app, Movement::Next, POS_WRAP_MATCH);
        move_and_assert_list_state!(app, Movement::Next, POS_WRAP_MATCH_MULTI_0_1);
        move_and_assert_list_state!(app, Movement::Next, POS_WRAP_MATCH_MULTI_0_2);
        move_and_assert_list_state!(app, Movement::Next, POS_WRAP_MATCH_MULTI_0_3);
        move_and_assert_list_state!(app, Movement::Next, POS_WRAP_MATCH_MULTI_0_4);
        move_and_assert_list_state!(app, Movement::Next, POS_WRAP_MATCH_MULTI_0_5);
        move_and_assert_list_state!(app, Movement::Next, POS_WRAP_MATCH_MULTI_0_6);
        move_and_assert_list_state!(app, Movement::Next, POS_WRAP_MATCH_MULTI_0_7);
        move_and_assert_list_state!(app, Movement::Next, POS_WRAP_END);
        move_and_assert_list_state!(app, Movement::Next, POS_WRAP_END);
        move_and_assert_list_state!(app, Movement::Prev, POS_WRAP_MATCH_MULTI_0_7);
        move_and_assert_list_state!(app, Movement::Prev, POS_WRAP_MATCH_MULTI_0_6);
        move_and_assert_list_state!(app, Movement::Prev, POS_WRAP_MATCH_MULTI_0_5);
        move_and_assert_list_state!(app, Movement::Prev, POS_WRAP_MATCH_MULTI_0_4);
        move_and_assert_list_state!(app, Movement::Prev, POS_WRAP_MATCH_MULTI_0_3);
        move_and_assert_list_state!(app, Movement::Prev, POS_WRAP_MATCH_MULTI_0_2);
        move_and_assert_list_state!(app, Movement::Prev, POS_WRAP_MATCH_MULTI_0_1);
        move_and_assert_list_state!(app, Movement::Prev, POS_WRAP_MATCH);
        move_and_assert_list_state!(app, Movement::Prev, POS_WRAP_BEGIN);
        move_and_assert_list_state!(app, Movement::Prev, POS_WRAP_BEGIN);
    }

    // NOTE: this test ensures that the indicator position is correct for matches that start on the
    // _first_ character of the next line
    #[test]
    fn movement_line_wrapping_at_end() {
        let rect = Rect::new(0, 0, 145, 24);
        let mut app = new_app_line_wrapping();
        assert_list_state!(app, POS_WRAP_BEGIN);
        move_and_assert_list_state!(app, Movement::Next, (2, 0, 2), rect);
        move_and_assert_list_state!(app, Movement::Next, (3, 0, 3), rect);
        move_and_assert_list_state!(app, Movement::Next, (3, 1, 3), rect);
        move_and_assert_list_state!(app, Movement::Next, (3, 2, 3), rect);
        move_and_assert_list_state!(app, Movement::Next, (3, 3, 3), rect);
        move_and_assert_list_state!(app, Movement::Next, (3, 4, 3), rect);
        move_and_assert_list_state!(app, Movement::Next, (3, 5, 4), rect);
        move_and_assert_list_state!(app, Movement::Next, (3, 6, 4), rect);
        move_and_assert_list_state!(app, Movement::Next, (4, 0, 5), rect);
        move_and_assert_list_state!(app, Movement::Next, (4, 0, 5), rect);
        move_and_assert_list_state!(app, Movement::Prev, (3, 6, 4), rect);
        move_and_assert_list_state!(app, Movement::Prev, (3, 5, 4), rect);
        move_and_assert_list_state!(app, Movement::Prev, (3, 4, 3), rect);
        move_and_assert_list_state!(app, Movement::Prev, (3, 3, 3), rect);
        move_and_assert_list_state!(app, Movement::Prev, (3, 2, 3), rect);
        move_and_assert_list_state!(app, Movement::Prev, (3, 1, 3), rect);
        move_and_assert_list_state!(app, Movement::Prev, (3, 0, 3), rect);
        move_and_assert_list_state!(app, Movement::Prev, (2, 0, 2), rect);
        move_and_assert_list_state!(app, Movement::Prev, POS_WRAP_BEGIN, rect);
        move_and_assert_list_state!(app, Movement::Prev, POS_WRAP_BEGIN, rect);
    }

    #[test]
    fn movement_next_and_prev() {
        let mut app = new_app_multiple_files();
        assert_list_state!(app, POS_1_BEGIN);
        move_and_assert_list_state!(app, Movement::Next, POS_1_MATCH_0_0);
        move_and_assert_list_state!(app, Movement::Next, POS_1_MATCH_0_1);
        move_and_assert_list_state!(app, Movement::Next, POS_1_MATCH_1_0);
        move_and_assert_list_state!(app, Movement::Next, POS_1_MATCH_1_1);
        move_and_assert_list_state!(app, Movement::Next, POS_2_BEGIN);
        move_and_assert_list_state!(app, Movement::Next, POS_2_MATCH_MULTILINE_0_0);
        move_and_assert_list_state!(app, Movement::Next, POS_2_MATCH_MULTILINE_0_1);
        move_and_assert_list_state!(app, Movement::Next, POS_3_BEGIN);
        move_and_assert_list_state!(app, Movement::Next, POS_3_MATCH_0_0);
        move_and_assert_list_state!(app, Movement::Next, POS_3_MATCH_0_1);
        move_and_assert_list_state!(app, Movement::Next, POS_3_MATCH_1_0);
        move_and_assert_list_state!(app, Movement::Next, POS_3_MATCH_1_1);
        move_and_assert_list_state!(app, Movement::Next, POS_4_BEGIN);
        move_and_assert_list_state!(app, Movement::Next, POS_4_MATCH_MULTILINE_0_0);
        move_and_assert_list_state!(app, Movement::Next, POS_4_MATCH_MULTILINE_0_1);
        move_and_assert_list_state!(app, Movement::Next, POS_4_END);
        move_and_assert_list_state!(app, Movement::Next, POS_4_END);
        move_and_assert_list_state!(app, Movement::Prev, POS_4_MATCH_MULTILINE_0_1);
        move_and_assert_list_state!(app, Movement::Prev, POS_4_MATCH_MULTILINE_0_0);
        move_and_assert_list_state!(app, Movement::Prev, POS_4_BEGIN);
        move_and_assert_list_state!(app, Movement::Prev, POS_3_MATCH_1_1);
        move_and_assert_list_state!(app, Movement::Prev, POS_3_MATCH_1_0);
        move_and_assert_list_state!(app, Movement::Prev, POS_3_MATCH_0_1);
        move_and_assert_list_state!(app, Movement::Prev, POS_3_MATCH_0_0);
        move_and_assert_list_state!(app, Movement::Prev, POS_3_BEGIN);
        move_and_assert_list_state!(app, Movement::Prev, POS_2_MATCH_MULTILINE_0_1);
        move_and_assert_list_state!(app, Movement::Prev, POS_2_MATCH_MULTILINE_0_0);
        move_and_assert_list_state!(app, Movement::Prev, POS_2_BEGIN);
        move_and_assert_list_state!(app, Movement::Prev, POS_1_MATCH_1_1);
        move_and_assert_list_state!(app, Movement::Prev, POS_1_MATCH_1_0);
        move_and_assert_list_state!(app, Movement::Prev, POS_1_MATCH_0_1);
        move_and_assert_list_state!(app, Movement::Prev, POS_1_MATCH_0_0);
        move_and_assert_list_state!(app, Movement::Prev, POS_1_BEGIN);
        move_and_assert_list_state!(app, Movement::Prev, POS_1_BEGIN);
    }

    #[test]
    fn movement_nextline_and_prevline() {
        let mut app = new_app_multiple_files();
        assert_list_state!(app, POS_1_BEGIN);
        move_and_assert_list_state!(app, Movement::NextLine, POS_1_MATCH_0_0);
        move_and_assert_list_state!(app, Movement::NextLine, POS_1_MATCH_1_0);
        move_and_assert_list_state!(app, Movement::NextLine, POS_2_BEGIN);
        move_and_assert_list_state!(app, Movement::NextLine, POS_2_MATCH_MULTILINE_0_0);
        move_and_assert_list_state!(app, Movement::NextLine, POS_3_BEGIN);
        move_and_assert_list_state!(app, Movement::NextLine, POS_3_MATCH_0_0);
        move_and_assert_list_state!(app, Movement::NextLine, POS_3_MATCH_1_0);
        move_and_assert_list_state!(app, Movement::NextLine, POS_4_BEGIN);
        move_and_assert_list_state!(app, Movement::NextLine, POS_4_MATCH_MULTILINE_0_0);
        move_and_assert_list_state!(app, Movement::NextLine, POS_4_END);
        move_and_assert_list_state!(app, Movement::NextLine, POS_4_END);
        move_and_assert_list_state!(app, Movement::PrevLine, POS_4_MATCH_MULTILINE_0_0);
        move_and_assert_list_state!(app, Movement::PrevLine, POS_4_BEGIN);
        move_and_assert_list_state!(app, Movement::PrevLine, POS_3_MATCH_1_0);
        move_and_assert_list_state!(app, Movement::PrevLine, POS_3_MATCH_0_0);
        move_and_assert_list_state!(app, Movement::PrevLine, POS_3_BEGIN);
        move_and_assert_list_state!(app, Movement::PrevLine, POS_2_MATCH_MULTILINE_0_0);
        move_and_assert_list_state!(app, Movement::PrevLine, POS_2_BEGIN);
        move_and_assert_list_state!(app, Movement::PrevLine, POS_1_MATCH_1_0);
        move_and_assert_list_state!(app, Movement::PrevLine, POS_1_MATCH_0_0);
        move_and_assert_list_state!(app, Movement::PrevLine, POS_1_BEGIN);
        move_and_assert_list_state!(app, Movement::PrevLine, POS_1_BEGIN);
    }

    #[test]
    fn movement_nextfile_and_prevfile() {
        let mut app = new_app_multiple_files();
        assert_list_state!(app, POS_1_BEGIN);
        move_and_assert_list_state!(app, Movement::NextFile, POS_2_BEGIN);
        move_and_assert_list_state!(app, Movement::NextFile, POS_3_BEGIN);
        move_and_assert_list_state!(app, Movement::NextFile, POS_4_BEGIN);
        move_and_assert_list_state!(app, Movement::NextFile, POS_4_END);
        move_and_assert_list_state!(app, Movement::NextFile, POS_4_END);
        move_and_assert_list_state!(app, Movement::PrevFile, POS_4_BEGIN);
        move_and_assert_list_state!(app, Movement::PrevFile, POS_3_BEGIN);
        move_and_assert_list_state!(app, Movement::PrevFile, POS_2_BEGIN);
        move_and_assert_list_state!(app, Movement::PrevFile, POS_1_BEGIN);
        move_and_assert_list_state!(app, Movement::PrevFile, POS_1_BEGIN);
    }

    #[test]
    fn movement_forward_1_and_backward_1() {
        let mut app = new_app_multiple_files();
        assert_list_state!(app, POS_1_BEGIN);
        move_and_assert_list_state!(app, Movement::Forward(1), POS_1_MATCH_0_0);
        move_and_assert_list_state!(app, Movement::Forward(1), POS_1_MATCH_1_0);
        move_and_assert_list_state!(app, Movement::Forward(1), POS_2_BEGIN);
        move_and_assert_list_state!(app, Movement::Forward(1), POS_2_MATCH_MULTILINE_0_0);
        move_and_assert_list_state!(app, Movement::Forward(1), POS_3_BEGIN);
        move_and_assert_list_state!(app, Movement::Forward(1), POS_3_MATCH_0_0);
        move_and_assert_list_state!(app, Movement::Forward(1), POS_3_MATCH_1_0);
        move_and_assert_list_state!(app, Movement::Forward(1), POS_4_BEGIN);
        move_and_assert_list_state!(app, Movement::Forward(1), POS_4_MATCH_MULTILINE_0_0);
        move_and_assert_list_state!(app, Movement::Forward(1), POS_4_END);
        move_and_assert_list_state!(app, Movement::Forward(1), POS_4_END);
        move_and_assert_list_state!(app, Movement::Backward(1), POS_4_MATCH_MULTILINE_0_0);
        move_and_assert_list_state!(app, Movement::Backward(1), POS_4_BEGIN);
        move_and_assert_list_state!(app, Movement::Backward(1), POS_3_MATCH_1_0);
        move_and_assert_list_state!(app, Movement::Backward(1), POS_3_MATCH_0_0);
        move_and_assert_list_state!(app, Movement::Backward(1), POS_3_BEGIN);
        move_and_assert_list_state!(app, Movement::Backward(1), POS_2_MATCH_MULTILINE_0_0);
        move_and_assert_list_state!(app, Movement::Backward(1), POS_2_BEGIN);
        move_and_assert_list_state!(app, Movement::Backward(1), POS_1_MATCH_1_0);
        move_and_assert_list_state!(app, Movement::Backward(1), POS_1_MATCH_0_0);
        move_and_assert_list_state!(app, Movement::Backward(1), POS_1_BEGIN);
        move_and_assert_list_state!(app, Movement::Backward(1), POS_1_BEGIN);
    }

    #[test]
    fn movement_forward_5_and_backward_5() {
        let mut app = new_app_multiple_files();
        assert_list_state!(app, POS_1_BEGIN);
        move_and_assert_list_state!(app, Movement::Forward(5), POS_2_BEGIN);
        move_and_assert_list_state!(app, Movement::Forward(5), POS_3_MATCH_1_0);
        move_and_assert_list_state!(app, Movement::Forward(5), POS_4_END);
        move_and_assert_list_state!(app, Movement::Forward(5), POS_4_END);
        move_and_assert_list_state!(app, Movement::Backward(5), POS_3_MATCH_1_0);
        move_and_assert_list_state!(app, Movement::Backward(5), POS_2_MATCH_MULTILINE_0_0);
        move_and_assert_list_state!(app, Movement::Backward(5), POS_1_MATCH_0_0);
        move_and_assert_list_state!(app, Movement::Backward(5), POS_1_BEGIN);
        move_and_assert_list_state!(app, Movement::Backward(5), POS_1_BEGIN);
    }

    #[test]
    fn movement_forward_100_and_backward_100() {
        let mut app = new_app_multiple_files();
        assert_list_state!(app, POS_1_BEGIN);
        move_and_assert_list_state!(app, Movement::Forward(100), POS_4_END);
        move_and_assert_list_state!(app, Movement::Forward(100), POS_4_END);
        move_and_assert_list_state!(app, Movement::Backward(100), POS_1_BEGIN);
        move_and_assert_list_state!(app, Movement::Backward(100), POS_1_BEGIN);
    }

    // cursor position when inputting replacement text

    use KeyCode::*;

    macro_rules! key {
        ($code:expr) => {
            key!($code, modifiers = KeyModifiers::empty())
        };
        ($code:expr, modifiers = $modifiers:expr) => {
            Event::Key(KeyEvent::new($code, $modifiers))
        };
        ($code:expr, kind = $kind:expr) => {
            Event::Key({
                let mut key = KeyEvent::new($code, KeyModifiers::empty());
                key.kind = $kind;
                key
            })
        };
    }

    macro_rules! send_key {
        ($app:expr, $key:expr) => {
            $app.on_event(Rect::new(0, 0, 80, 24), $key).unwrap();
        };
    }

    macro_rules! send_key_assert {
        ($app:expr, $key:expr, $input:expr, $pos:expr) => {
            send_key!($app, $key);
            assert_eq!(
                $app.ui_state,
                AppUiState::InputReplacement($input.into(), $pos)
            );
        };
    }

    #[test]
    fn works_with_other_key_event_kinds() {
        let mut app = new_app();

        // enter insert mode
        send_key_assert!(app, key!(Enter, kind = KeyEventKind::Press), "", 0);
        send_key_assert!(app, key!(Enter, kind = KeyEventKind::Repeat), "", 0);
        send_key_assert!(app, key!(Enter, kind = KeyEventKind::Release), "", 0);

        // insert text
        send_key_assert!(app, key!(Char('a'), kind = KeyEventKind::Press), "a", 1);
        send_key_assert!(app, key!(Char('a'), kind = KeyEventKind::Repeat), "a", 1);
        send_key_assert!(app, key!(Char('a'), kind = KeyEventKind::Release), "a", 1);
    }

    #[test]
    fn input_replacement() {
        let mut app = new_app();

        // enter insert mode
        send_key_assert!(app, key!(Enter), "", 0);

        // check with no text
        send_key_assert!(app, key!(Left), "", 0);
        send_key_assert!(app, key!(Right), "", 0);

        // insert text
        send_key_assert!(app, key!(Char('e')), "e", 1);
        send_key_assert!(app, key!(Right), "e", 1);
        send_key_assert!(app, key!(Left), "e", 0);
        send_key_assert!(app, key!(Left), "e", 0);
        send_key_assert!(app, key!(Char('r')), "re", 1);
        send_key_assert!(app, key!(Right), "re", 2);
        send_key_assert!(app, key!(Char('p')), "rep", 3);

        // insert emoji
        send_key_assert!(app, key!(Char('🎉')), "rep🎉", 4);
        send_key_assert!(app, key!(Left), "rep🎉", 3);
        send_key_assert!(app, key!(Left), "rep🎉", 2);
        send_key_assert!(app, key!(Left), "rep🎉", 1);
        send_key_assert!(app, key!(Left), "rep🎉", 0);
        send_key_assert!(app, key!(Char('🎉')), "🎉rep🎉", 1);
        send_key_assert!(app, key!(Left), "🎉rep🎉", 0);
        send_key_assert!(app, key!(Left), "🎉rep🎉", 0);
        send_key_assert!(app, key!(Right), "🎉rep🎉", 1);
        send_key_assert!(app, key!(Right), "🎉rep🎉", 2);
        send_key_assert!(app, key!(Char('🎉')), "🎉r🎉ep🎉", 3);
        send_key_assert!(app, key!(Right), "🎉r🎉ep🎉", 4);
        send_key_assert!(app, key!(Right), "🎉r🎉ep🎉", 5);
        send_key_assert!(app, key!(Right), "🎉r🎉ep🎉", 6);
        send_key_assert!(app, key!(Right), "🎉r🎉ep🎉", 6);

        // delete
        send_key_assert!(app, key!(Delete), "🎉r🎉ep🎉", 6);
        send_key_assert!(app, key!(Left), "🎉r🎉ep🎉", 5);
        send_key_assert!(app, key!(Delete), "🎉r🎉ep", 5);
        send_key_assert!(app, key!(Left), "🎉r🎉ep", 4);
        send_key_assert!(app, key!(Left), "🎉r🎉ep", 3);
        send_key_assert!(app, key!(Left), "🎉r🎉ep", 2);
        send_key_assert!(app, key!(Left), "🎉r🎉ep", 1);
        send_key_assert!(app, key!(Left), "🎉r🎉ep", 0);
        send_key_assert!(app, key!(Delete), "r🎉ep", 0);
        send_key_assert!(app, key!(Delete), "🎉ep", 0);
        send_key_assert!(app, key!(Delete), "ep", 0);

        // backspace
        send_key_assert!(app, key!(Backspace), "ep", 0);
        send_key_assert!(app, key!(Right), "ep", 1);
        send_key_assert!(app, key!(Backspace), "p", 0);
        send_key_assert!(app, key!(Right), "p", 1);
        send_key_assert!(app, key!(Backspace), "", 0);
        send_key_assert!(app, key!(Backspace), "", 0);

        // fill up with text again
        send_key_assert!(app, key!(Char('r')), "r", 1);
        send_key_assert!(app, key!(Char('e')), "re", 2);
        send_key_assert!(app, key!(Char('p')), "rep", 3);
        send_key_assert!(app, key!(Char('g')), "repg", 4);
        send_key_assert!(app, key!(Char('r')), "repgr", 5);
        send_key_assert!(app, key!(Char('e')), "repgre", 6);
        send_key_assert!(app, key!(Char('p')), "repgrep", 7);

        // check movement keys
        send_key_assert!(app, key!(Home), "repgrep", 0);
        send_key_assert!(app, key!(End), "repgrep", 7);
        send_key_assert!(app, key!(PageUp), "repgrep", 0);
        send_key_assert!(app, key!(PageDown), "repgrep", 7);

        // move to next mode
        send_key!(app, key!(Char('s'), modifiers = KeyModifiers::CONTROL));
        assert_eq!(
            app.ui_state,
            AppUiState::ConfirmReplacement("repgrep".into(), 7)
        );

        // move back and check pos
        send_key_assert!(app, key!(Esc), "repgrep", 7);
        send_key_assert!(app, key!(Left), "repgrep", 6);
        send_key!(app, key!(Char('s'), modifiers = KeyModifiers::CONTROL));
        assert_eq!(
            app.ui_state,
            AppUiState::ConfirmReplacement("repgrep".into(), 6)
        );
        send_key_assert!(app, key!(Esc), "repgrep", 6);

        // move back again
        send_key!(app, key!(Esc));
        assert_eq!(app.ui_state, AppUiState::SelectMatches);

        // and back to input
        send_key_assert!(app, key!(Enter), "", 0);
    }
}
