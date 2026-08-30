use ratatui::style::{Color, Modifier, Style};

/// Styles used throughout the UI.
///
/// Foreground and background colours are left to the terminal wherever possible so the UI adapts
/// to both light and dark terminal themes.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub normal: Style,
    pub muted: Style,
    pub selected: Style,
    pub file: Style,
    pub file_selected: Style,
    pub matched: Style,
    pub matched_selected: Style,
    pub matched_disabled: Style,
    pub matched_disabled_selected: Style,
    pub removed: Style,
    pub inserted: Style,
    pub status: Style,
    pub emphasis: Style,
    pub help_heading: Style,
    pub help_keys: Style,
    pub error: Style,
}

impl Default for Theme {
    fn default() -> Theme {
        let normal = Style::default();
        let muted = normal.add_modifier(Modifier::DIM);
        let selected = normal.add_modifier(Modifier::REVERSED);
        let matched = normal
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::REVERSED);
        let matched_disabled = normal.fg(Color::Red).add_modifier(Modifier::CROSSED_OUT);

        Theme {
            normal,
            muted,
            selected,
            file: normal.fg(Color::Magenta),
            file_selected: selected,
            matched,
            matched_selected: matched.add_modifier(Modifier::UNDERLINED),
            matched_disabled,
            matched_disabled_selected: matched_disabled
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::UNDERLINED),
            removed: normal.fg(Color::Red).add_modifier(Modifier::CROSSED_OUT),
            inserted: normal.fg(Color::Green),
            status: normal.add_modifier(Modifier::REVERSED),
            emphasis: normal.add_modifier(Modifier::BOLD),
            help_heading: normal.fg(Color::Magenta),
            help_keys: normal
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
            error: normal.fg(Color::Red),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_colours_are_used_for_ui_surfaces() {
        let theme = Theme::default();

        for style in [
            theme.normal,
            theme.muted,
            theme.selected,
            theme.file_selected,
            theme.status,
            theme.emphasis,
            theme.help_keys,
        ] {
            assert_eq!(style.fg, None);
            assert_eq!(style.bg, None);
        }
    }

    #[test]
    fn enabled_and_disabled_matches_have_distinct_styles() {
        let theme = Theme::default();

        assert_eq!(theme.matched.fg, Some(Color::Red));
        assert!(theme.matched.add_modifier.contains(Modifier::REVERSED));
        assert!(theme.matched.add_modifier.contains(Modifier::BOLD));

        assert_eq!(theme.matched_disabled.fg, Some(Color::Red));
        assert!(
            theme
                .matched_disabled
                .add_modifier
                .contains(Modifier::CROSSED_OUT)
        );
        assert!(
            !theme
                .matched_disabled
                .add_modifier
                .contains(Modifier::REVERSED)
        );

        assert!(
            theme
                .matched_selected
                .add_modifier
                .contains(Modifier::UNDERLINED)
        );
        assert!(
            theme
                .matched_disabled_selected
                .add_modifier
                .contains(Modifier::UNDERLINED)
        );
    }
}
