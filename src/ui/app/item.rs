use tui::style::{Color, Style};
use tui::widgets::Text;

use crate::rg::de::RgMessageType;

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

  pub fn match_count(&self) -> usize {
    match &self.rg_message_type {
      RgMessageType::Match { submatches, .. } => submatches.len(),
      _ => 0,
    }
  }

  pub fn to_text(&self, replacement: Option<&String>) -> Text {
    // TODO: handle non-UTF-8 text
    match &self.rg_message_type {
      RgMessageType::Begin { path, .. } => {
        Text::styled(path.lossy_utf8(), Style::default().fg(Color::Magenta))
      }
      RgMessageType::Context {
        lines, line_number, ..
      } => {
        let mut text = lines.lossy_utf8();
        if let Some(number) = line_number {
          text = format!("{}:{}", number, text);
        }

        Text::styled(text, Style::default().fg(Color::DarkGray))
      }
      RgMessageType::Match {
        lines,
        line_number,
        submatches,
        ..
      } => {
        // TODO: highlight matches (red) on line and replacements (green). Currently not possible.
        // See: https://github.com/fdehau/tui-rs/issues/315
        let mut style = Style::default();
        if !self.should_replace {
          style = style.fg(Color::Red);
        }

        let mut text = lines.lossy_utf8();
        // TODO: when we can highlight mid-text, don't replace the match, colour the match (submatch.text.lossy_utf8())
        // and add the replacement after.
        if self.should_replace {
          if let Some(replacement) = replacement {
            for submatch in submatches.iter().rev() {
              text.replace_range(submatch.range.clone(), replacement);
            }
          }
        }

        if let Some(number) = line_number {
          text = format!("{}:{}", number, text);
        }

        Text::styled(text, style)
      }
      RgMessageType::End { .. } => Text::raw(""),
      unexpected_type => panic!(
        "Unexpected enum variant, got {:?} and expected only Context or Match!",
        unexpected_type
      ),
    }
  }
}
