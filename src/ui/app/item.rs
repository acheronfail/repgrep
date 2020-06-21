use tui::style::{Color, Style};
use tui::widgets::Text;

use crate::rg::de::{ArbitraryData, RgMessageType};

#[derive(Debug, PartialEq, Eq)]
pub enum ItemKind {
  Begin,
  Context,
  Match,
  End,
  Summary,
}

pub struct Item {
  rg_message_type: RgMessageType,

  pub kind: ItemKind,

  pub should_replace: bool,
}

impl Item {
  pub fn new(rg_message_type: RgMessageType) -> Item {
    let kind = match rg_message_type {
      RgMessageType::Begin { .. } => ItemKind::Begin,
      RgMessageType::Match { .. } => ItemKind::Match,
      RgMessageType::Context { .. } => ItemKind::Context,
      RgMessageType::End { .. } => ItemKind::End,
      RgMessageType::Summary { .. } => ItemKind::Summary,
    };

    Item {
      rg_message_type,
      kind,
      should_replace: true,
    }
  }

  pub fn is_selectable(&self) -> bool {
    matches!(self.kind, ItemKind::Begin | ItemKind::Match)
  }

  pub fn to_text(&self) -> Text {
    // TODO: color line number, currently not possible
    // See: https://github.com/fdehau/tui-rs/issues/315
    let lines_as_string = |lines: &ArbitraryData, line_number: &Option<usize>| {
      let mut s = lines.lossy_utf8();
      if let Some(number) = line_number {
        s = format!("{}:{}", number, s);
      }

      s
    };

    // TODO: handle non-UTF-8 text
    match &self.rg_message_type {
      RgMessageType::Begin { path, .. } => Text::styled(
        format!("file: {}", path.lossy_utf8()),
        Style::default().fg(Color::Magenta),
      ),
      RgMessageType::Context {
        lines, line_number, ..
      } => Text::styled(
        lines_as_string(lines, line_number),
        Style::default().fg(Color::DarkGray),
      ),
      RgMessageType::Match {
        lines, line_number, ..
      } => {
        // TODO: highlight matches on line, currently not possible
        // See: https://github.com/fdehau/tui-rs/issues/315
        let mut style = Style::default();
        if !self.should_replace {
          style = style.fg(Color::Red);
        }

        Text::styled(lines_as_string(lines, line_number), style)
      }
      RgMessageType::End { .. } => Text::raw(""),
      unexpected_type => panic!(
        "Unexpected enum variant, got {:?} and expected only Context or Match!",
        unexpected_type
      ),
    }
  }
}
