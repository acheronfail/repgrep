use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::Result;
use tui::style::{Color, Style};
use tui::widgets::Text;

use crate::rg::de::{ArbitraryData, RgMessage, RgMessageKind, SubMatch};

// TODO: tests for Base64 decoding on separate platforms

/// Convert Base64 encoded data to an OsString on Unix platforms.
/// https://doc.rust-lang.org/std/ffi/index.html#on-unix
#[cfg(unix)]
fn base64_to_os_string(bytes: Vec<u8>) -> Result<OsString> {
  use std::os::unix::ffi::OsStringExt;
  Ok(OsString::from_vec(bytes))
}

/// Convert Base64 encoded data to an OsString on Windows platforms.
/// https://doc.rust-lang.org/std/ffi/index.html#on-windows
#[cfg(not(unix))]
fn base64_to_os_string(bytes: Vec<u8>) -> Result<OsString> {
  use safe_transmute::{transmute_many, try_copy, PedanticGuard};
  use std::os::windows::ffi::OsStringExt;

  // Transmute decoded Base64 bytes as UTF-16 since that's what underlying paths are on Windows.
  let bytes_u16 = try_copy!(transmute_many::<u16, PedanticGuard>(&bytes))?;
  OsString::from_wide(&bytes_u16)
}

#[derive(Debug, Clone)]
pub struct Item {
  rg_message: RgMessage,

  pub kind: RgMessageKind,
  pub should_replace: bool,
}

impl Item {
  pub fn new(rg_message: RgMessage) -> Item {
    let kind = match rg_message {
      RgMessage::Begin { .. } => RgMessageKind::Begin,
      RgMessage::End { .. } => RgMessageKind::End,
      RgMessage::Match { .. } => RgMessageKind::Match,
      RgMessage::Context { .. } => RgMessageKind::Context,
      RgMessage::Summary { .. } => RgMessageKind::Summary,
    };

    Item {
      rg_message,
      kind,
      should_replace: true,
    }
  }

  pub fn is_selectable(&self) -> bool {
    matches!(self.kind, RgMessageKind::Begin | RgMessageKind::Match)
  }

  pub fn offset(&self) -> Option<usize> {
    match &self.rg_message {
      RgMessage::End { binary_offset, .. } => *binary_offset,
      RgMessage::Match {
        absolute_offset, ..
      } => Some(*absolute_offset),
      _ => None,
    }
  }

  pub fn match_count(&self) -> usize {
    self
      .matches()
      .map(|submatches| submatches.len())
      .unwrap_or(0)
  }

  pub fn matches(&self) -> Option<&[SubMatch]> {
    match &self.rg_message {
      RgMessage::Match { submatches, .. } => Some(submatches),
      _ => None,
    }
  }

  pub fn path(&self) -> Option<PathBuf> {
    let path_data = match &self.rg_message {
      RgMessage::Begin { path, .. } => path,
      RgMessage::Match { path, .. } => path,
      RgMessage::Context { path, .. } => path,
      RgMessage::End { path, .. } => path,
      RgMessage::Summary { .. } => return None,
    };

    Some(match path_data {
      ArbitraryData::Text { text } => PathBuf::from(text),
      ArbitraryData::Base64 { bytes } => {
        // Decode the Base64 into u8 bytes.
        let data = match base64::decode(bytes) {
          Ok(data) => data,
          Err(e) => panic!("Error deserialising Base64 data: {}", e),
        };

        // Convert the bytes into an OsString.
        let os_string = match base64_to_os_string(data) {
          Ok(os_string) => os_string,
          Err(e) => panic!("Error transmuting Base64 data to OsString: {}", e),
        };

        PathBuf::from(os_string)
      }
    })
  }

  pub fn to_text(&self, replacement: Option<&str>) -> Text {
    // TODO: handle multiline matches
    match &self.rg_message {
      RgMessage::Begin { .. } => Text::styled(
        format!("{}", self.path().unwrap().display()),
        Style::default().fg(Color::Magenta),
      ),
      RgMessage::Context {
        lines, line_number, ..
      } => {
        let mut text = lines.lossy_utf8();
        if let Some(number) = line_number {
          text = format!("{}:{}", number, text);
        }

        Text::styled(text, Style::default().fg(Color::DarkGray))
      }
      RgMessage::Match {
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

        // TODO: when we can highlight mid-text, don't replace the match, colour the match (submatch.text.lossy_utf8())
        // and add the replacement after.
        // If we have a replacement, then perform the replacement on the bytes before encoding as UTF8.
        let mut bytes = lines.to_vec();
        if self.should_replace {
          if let Some(replacement) = replacement {
            let replacement = replacement.as_bytes().to_vec();
            for submatch in submatches.iter().rev() {
              bytes.splice(submatch.range.clone(), replacement.clone());
            }
          }
        }

        // Prepend the line number.
        let mut text: String = String::from_utf8_lossy(&bytes).to_string();
        if let Some(number) = line_number {
          text = format!("{}:{}", number, text);
        }

        Text::styled(text, style)
      }
      RgMessage::End { .. } => Text::raw(""),
      RgMessage::Summary { elapsed_total, .. } => {
        Text::raw(format!("Search duration: {}", elapsed_total.human))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use pretty_assertions::assert_eq;
  use tui::style::{Color, Style};
  use tui::widgets::Text;

  use crate::model::*;
  use crate::rg::de::test_utilities::*;
  use crate::rg::de::*;

  const RG_JSON_BEGIN: &str = r#"{"type":"begin","data":{"path":{"text":"src/model/item.rs"}}}"#;
  const RG_JSON_MATCH: &str = r#"{"type":"match","data":{"path":{"text":"src/model/item.rs"},"lines":{"text":"    Item::new(rg_msg)\n"},"line_number":197,"absolute_offset":5522,"submatches":[{"match":{"text":"rg_msg"},"start":14,"end":20}]}}"#;
  const RG_JSON_CONTEXT: &str = r#"{"type":"context","data":{"path":{"text":"src/model/item.rs"},"lines":{"text":"  }\n"},"line_number":198,"absolute_offset":5544,"submatches":[]}}"#;
  const RG_JSON_END: &str = r#"{"type":"end","data":{"path":{"text":"src/model/item.rs"},"binary_offset":null,"stats":{"elapsed":{"secs":0,"nanos":97924,"human":"0.000098s"},"searches":1,"searches_with_match":1,"bytes_searched":5956,"bytes_printed":674,"matched_lines":2,"matches":2}}}"#;
  const RG_JSON_SUMMARY: &str = r#"{"data":{"elapsed_total":{"human":"0.013911s","nanos":13911027,"secs":0},"stats":{"bytes_printed":3248,"bytes_searched":18789,"elapsed":{"human":"0.000260s","nanos":260276,"secs":0},"matched_lines":10,"matches":10,"searches":2,"searches_with_match":2}},"type":"summary"}"#;

  fn new_item(raw_json: &str) -> Item {
    let rg_msg = serde_json::from_str::<RgMessage>(raw_json).unwrap();
    Item::new(rg_msg)
  }

  #[test]
  fn item_kind_matches_rg_message_kind() {
    assert_eq!(new_item(RG_JSON_BEGIN).kind, RgMessageKind::Begin);
    assert_eq!(new_item(RG_JSON_MATCH).kind, RgMessageKind::Match);
    assert_eq!(new_item(RG_JSON_CONTEXT).kind, RgMessageKind::Context);
    assert_eq!(new_item(RG_JSON_END).kind, RgMessageKind::End);
    assert_eq!(new_item(RG_JSON_SUMMARY).kind, RgMessageKind::Summary);
  }

  #[test]
  fn only_match_and_begin_are_selectable() {
    assert_eq!(new_item(RG_JSON_BEGIN).is_selectable(), true);
    assert_eq!(new_item(RG_JSON_MATCH).is_selectable(), true);
    assert_eq!(new_item(RG_JSON_CONTEXT).is_selectable(), false);
    assert_eq!(new_item(RG_JSON_END).is_selectable(), false);
    assert_eq!(new_item(RG_JSON_SUMMARY).is_selectable(), false);
  }

  #[test]
  fn match_count() {
    assert_eq!(new_item(RG_JSON_BEGIN).match_count(), 0);
    assert_eq!(new_item(RG_JSON_MATCH).match_count(), 1);
    assert_eq!(new_item(RG_JSON_CONTEXT).match_count(), 0);
    assert_eq!(new_item(RG_JSON_END).match_count(), 0);
    assert_eq!(new_item(RG_JSON_SUMMARY).match_count(), 0);
  }

  #[test]
  fn matches() {
    assert_eq!(new_item(RG_JSON_BEGIN).matches(), None);
    assert_eq!(
      new_item(RG_JSON_MATCH).matches(),
      Some([SubMatch::new_text("rg_msg", 14..20)].as_ref())
    );
    assert_eq!(new_item(RG_JSON_CONTEXT).matches(), None);
    assert_eq!(new_item(RG_JSON_END).matches(), None);
    assert_eq!(new_item(RG_JSON_SUMMARY).matches(), None);
  }

  #[test]
  fn offset() {
    assert_eq!(new_item(RG_JSON_BEGIN).offset(), None);
    assert_eq!(new_item(RG_JSON_MATCH).offset(), Some(5522));
    assert_eq!(new_item(RG_JSON_CONTEXT).offset(), None);
    assert_eq!(new_item(RG_JSON_END).offset(), None);
    assert_eq!(new_item(RG_JSON_SUMMARY).offset(), None);
  }

  #[test]
  fn binary_offset() {
    let item = new_item(
      r#"{"type":"end","data":{"path":{"text":"src/model/item.rs"},"binary_offset":1234,"stats":{"elapsed":{"secs":0,"nanos":97924,"human":"0.000098s"},"searches":1,"searches_with_match":1,"bytes_searched":5956,"bytes_printed":674,"matched_lines":2,"matches":2}}}"#,
    );
    assert_eq!(item.offset(), Some(1234));
  }

  #[test]
  fn path_with_text() {
    let path = PathBuf::from("src/model/item.rs");
    assert_eq!(new_item(RG_JSON_BEGIN).path().as_ref(), Some(&path));
    assert_eq!(new_item(RG_JSON_MATCH).path().as_ref(), Some(&path));
    assert_eq!(new_item(RG_JSON_CONTEXT).path().as_ref(), Some(&path));
    assert_eq!(new_item(RG_JSON_END).path().as_ref(), Some(&path));
    assert_eq!(new_item(RG_JSON_SUMMARY).path().as_ref(), None);
  }

  // TODO: write a similar test for Windows systems
  #[test]
  #[cfg(unix)]
  fn path_with_base64() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    // Here, the values 0x66 and 0x6f correspond to 'f' and 'o'
    // respectively. The value 0x80 is a lone continuation byte, invalid
    // in a UTF-8 sequence.
    let invalid_utf8_name_bytes = [0x66, 0x6f, 0x80, 0x6f];
    let invalid_utf8_name = OsStr::from_bytes(&invalid_utf8_name_bytes[..]);
    let invalid_utf8_path = PathBuf::from(invalid_utf8_name);

    let new_item_path_base64 = |kind| {
      Item::new(
        RgMessageBuilder::new(kind)
          .with_path_base64(base64::encode(&invalid_utf8_name_bytes))
          .with_lines_text("foo bar baz")
          .with_submatches(vec![SubMatch::new_text("foo", 0..3)])
          .with_stats(Stats::new())
          .with_elapsed_total(Duration::new())
          .with_offset(0)
          .build(),
      )
    };

    assert_eq!(
      new_item_path_base64(RgMessageKind::Begin).path().as_ref(),
      Some(&invalid_utf8_path)
    );
    assert_eq!(
      new_item_path_base64(RgMessageKind::Match).path().as_ref(),
      Some(&invalid_utf8_path)
    );
    assert_eq!(
      new_item_path_base64(RgMessageKind::Context).path().as_ref(),
      Some(&invalid_utf8_path)
    );
    assert_eq!(
      new_item_path_base64(RgMessageKind::End).path().as_ref(),
      Some(&invalid_utf8_path)
    );
    assert_eq!(
      new_item_path_base64(RgMessageKind::Summary).path().as_ref(),
      None
    );
  }

  #[test]
  fn to_text_with_text() {
    let s = Style::default();

    // Without replacement.
    assert_eq!(
      new_item(RG_JSON_BEGIN).to_text(None),
      Text::styled("src/model/item.rs", s.fg(Color::Magenta))
    );
    assert_eq!(
      new_item(RG_JSON_MATCH).to_text(None),
      Text::styled("197:    Item::new(rg_msg)\n", s)
    );
    assert_eq!(
      new_item(RG_JSON_CONTEXT).to_text(None),
      Text::styled("198:  }\n", s.fg(Color::DarkGray))
    );
    assert_eq!(new_item(RG_JSON_END).to_text(None), Text::raw(""));
    assert_eq!(
      new_item(RG_JSON_SUMMARY).to_text(None),
      Text::raw("Search duration: 0.013911s")
    );

    // With replacement.
    let replacement = "foobar";
    assert_eq!(
      new_item(RG_JSON_BEGIN).to_text(Some(replacement)),
      Text::styled("src/model/item.rs", s.fg(Color::Magenta))
    );
    assert_eq!(
      new_item(RG_JSON_MATCH).to_text(Some(replacement)),
      Text::styled("197:    Item::new(foobar)\n", s)
    );
    assert_eq!(
      new_item(RG_JSON_CONTEXT).to_text(Some(replacement)),
      Text::styled("198:  }\n", s.fg(Color::DarkGray))
    );
    assert_eq!(
      new_item(RG_JSON_END).to_text(Some(replacement)),
      Text::raw("")
    );
    assert_eq!(
      new_item(RG_JSON_SUMMARY).to_text(Some(replacement)),
      Text::raw("Search duration: 0.013911s")
    );
  }

  #[test]
  fn to_text_with_base64_lossy() {
    // The following types are skipped because:
    // Begin:   already tested via the `path_with_base64` test.
    // End:     already tested via the `path_with_base64` test.
    // Summary: doesn't include an `ArbitraryData` struct.

    let b64_json_match = r#"{"type":"match","data":{"path":{"text":"src/model/item.rs"},"lines":{"bytes":"ICAgIEl0ZW06Ov9uZXcocmdfbXNnKQo="},"line_number":197,"absolute_offset":5522,"submatches":[{"match":{"text":"rg_msg"},"start":15,"end":21}]}}"#;
    let b64_json_context = r#"{"type":"context","data":{"path":{"text":"src/model/item.rs"},"lines":{"bytes":"ICD/fQo="},"line_number":198,"absolute_offset":5544,"submatches":[]}}"#;

    // Since we don't read the entire file when we view the results, we expect the UTF8 replacement character.
    // Without replacement.
    let s = Style::default();
    assert_eq!(
      new_item(b64_json_match).to_text(None),
      Text::styled("197:    Item::�new(rg_msg)\n", s)
    );
    assert_eq!(
      new_item(b64_json_context).to_text(None),
      Text::styled("198:  �}\n", s.fg(Color::DarkGray))
    );

    // With replacement.
    let replacement = "foobar";
    assert_eq!(
      new_item(b64_json_match).to_text(Some(replacement)),
      Text::styled("197:    Item::�new(foobar)\n", s)
    );
    assert_eq!(
      new_item(b64_json_context).to_text(Some(replacement)),
      Text::styled("198:  �}\n", s.fg(Color::DarkGray))
    );
  }
}
