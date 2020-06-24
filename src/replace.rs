use std::fs::{self, OpenOptions};
use std::io::{Read, Write};

use anyhow::Result;
use chardet::charset2encoding;
use encoding::label::encoding_from_whatwg_label;
use encoding::EncoderTrap;

use crate::model::{ReplacementCriteria, ReplacementResult};
use crate::rg::de::{RgMessageKind, SubMatch};

const BOM_UTF8: [u8; 3] = [0xEF, 0xBB, 0xBF];
const BOM_UTF16LE: [u8; 2] = [0xFF, 0xFE];
const BOM_UTF16BE: [u8; 2] = [0xFE, 0xFF];

pub fn perform_replacements(criteria: ReplacementCriteria) -> Result<ReplacementResult> {
  Ok(
    criteria
      .items
      .iter()
      // Iterate backwards so the offset doesn't change as we make replacements.
      .rev()
      // The only item kind we replace is the Match kind.
      .filter(|item| matches!(item.kind, RgMessageKind::Match) && item.should_replace)
      // Perform the replacement on each match.
      // TODO: better error handling and messaging to the user when any of this fails
      .fold(ReplacementResult::new(&criteria.text), |mut res, item| {
        let file_path = item.path().expect("match item did not have a path!");

        // TODO: don't read file completely into memory, but use a buffered approach instead
        let mut file_contents = vec![];
        OpenOptions::new()
          .read(true)
          .open(&file_path)
          .unwrap()
          .read_to_end(&mut file_contents)
          .unwrap();

        // Guess the file's encoding. We only use the encoding if the confidence is greater than 80%.
        // TODO: read `rg`'s command line and check if encoding was passed
        let (encoding, confidence, _) = chardet::detect(&file_contents);
        let (encoder, replacement) = if confidence > 0.80 {
          let encoder = encoding_from_whatwg_label(charset2encoding(&encoding)).unwrap();
          let encoded_replacement = encoder.encode(&criteria.text, EncoderTrap::Ignore).unwrap();
          (Some(encoder), encoded_replacement)
        } else {
          (None, criteria.text.as_bytes().to_vec())
        };

        // Replace matches within the file contents with the given `replacement` string.
        let replaced_matches = item.matches().map_or_else(
          || vec![],
          |submatches| {
            let mut offset = item.offset().unwrap_or(0);

            // Increase offset to take into account the BOM if it exists.
            if (encoding == "UTF-16LE" && file_contents[0..2] == BOM_UTF16LE)
              || (encoding == "UTF-16BE" && file_contents[0..2] == BOM_UTF16BE)
            {
              offset += 2;
            } else if encoding == "UTF-8" && file_contents[0..3] == BOM_UTF8 {
              offset += 3;
            }

            // Iterate backwards so the offset doesn't change as we make replacements.
            submatches
              .iter()
              .rev()
              .map(|submatch| {
                let SubMatch { text, range } = submatch;
                let removed_bytes = file_contents
                  .splice(
                    (offset + range.start)..(offset + range.end),
                    replacement.clone(),
                  )
                  .collect::<Vec<_>>();

                // Assert that the portion we replaced matches the matched portion.
                let matched_bytes =
                  encoder.map_or_else(|| text.to_vec(), |e| text.to_vec_with_encoding(e));

                assert_eq!(
                  &matched_bytes,
                  &removed_bytes,
                  "Matched bytes do not match bytes to replace in {}@{}!",
                  file_path.display(),
                  offset + range.start,
                );

                text.lossy_utf8()
              })
              .collect()
          },
        );

        // Write modified string into a temporary file.
        let temp_file_path = &file_path.with_extension("rgr");
        OpenOptions::new()
          .create(true)
          .write(true)
          .truncate(true)
          .open(&temp_file_path)
          .unwrap()
          .write_all(&file_contents)
          .unwrap();

        // Overwrite the original file with the patched temp file.
        fs::rename(temp_file_path, &file_path).unwrap();

        res.add_replacement(
          &file_path,
          &replaced_matches,
          match encoder {
            Some(_) => encoding,
            None => "utf-8".to_owned(),
          },
        );
        res
      }),
  )
}

#[cfg(test)]
mod tests {
  use std::fs::{self, OpenOptions};
  use std::io::{Read, Write};
  use std::path::PathBuf;

  use pretty_assertions::assert_eq;
  use tempfile::{tempdir, NamedTempFile};

  use crate::model::*;
  use crate::replace::perform_replacements;
  use crate::rg::de::test_utilities::RgMessageBuilder;
  use crate::rg::de::{Duration, RgMessageKind, Stats, SubMatch};

  fn temp_rg_msg(
    mut f: &NamedTempFile,
    offset: usize,
    lines: impl AsRef<str>,
    submatches: Vec<SubMatch>,
  ) -> Item {
    f.write_all(lines.as_ref().as_bytes()).unwrap();

    Item::new(
      RgMessageBuilder::new(RgMessageKind::Match)
        .with_path_text(f.path().to_string_lossy().to_string())
        .with_lines_text(lines.as_ref().to_string())
        .with_submatches(submatches)
        .with_offset(offset)
        .build(),
    )
  }

  #[test]
  fn it_performs_replacements_only_on_match_items() {
    let text = "foo bar baz";
    let build_item = |kind, mut f: &NamedTempFile| {
      f.write_all(text.as_bytes()).unwrap();
      Item::new(
        RgMessageBuilder::new(kind)
          .with_path_text(f.path().to_string_lossy())
          .with_lines_text(text)
          .with_submatches(vec![SubMatch::new_text("foo", 0..3)])
          .with_stats(Stats::new())
          .with_elapsed_total(Duration::new())
          .with_offset(0)
          .build(),
      )
    };

    let f1 = NamedTempFile::new().unwrap();
    let f2 = NamedTempFile::new().unwrap();
    let f3 = NamedTempFile::new().unwrap();
    let f4 = NamedTempFile::new().unwrap();
    let f5 = NamedTempFile::new().unwrap();

    let items = vec![
      build_item(RgMessageKind::Begin, &f1),
      build_item(RgMessageKind::Context, &f2),
      build_item(RgMessageKind::Match, &f3),
      build_item(RgMessageKind::End, &f4),
      build_item(RgMessageKind::Summary, &f5),
    ];

    let result = perform_replacements(ReplacementCriteria::new("NEW_VALUE", items)).unwrap();
    assert_eq!(result.replacements.len(), 1);

    assert_eq!(fs::read_to_string(f1.path()).unwrap(), text);
    assert_eq!(fs::read_to_string(f2.path()).unwrap(), text);
    assert_eq!(fs::read_to_string(f3.path()).unwrap(), "NEW_VALUE bar baz");
    assert_eq!(fs::read_to_string(f4.path()).unwrap(), text);
    assert_eq!(fs::read_to_string(f5.path()).unwrap(), text);
  }

  #[test]
  fn it_performs_replacements_in_separate_files() {
    let f1 = NamedTempFile::new().unwrap();
    let f2 = NamedTempFile::new().unwrap();
    let f3 = NamedTempFile::new().unwrap();

    let items = vec![
      temp_rg_msg(&f1, 0, "foo bar baz", vec![SubMatch::new_text("foo", 0..3)]),
      temp_rg_msg(&f2, 0, "baz foo bar", vec![SubMatch::new_text("foo", 4..7)]),
      temp_rg_msg(
        &f3,
        0,
        "bar baz foo",
        vec![SubMatch::new_text("foo", 8..11)],
      ),
    ];

    perform_replacements(ReplacementCriteria::new("NEW_VALUE", items)).unwrap();
    assert_eq!(fs::read_to_string(f1.path()).unwrap(), "NEW_VALUE bar baz");
    assert_eq!(fs::read_to_string(f2.path()).unwrap(), "baz NEW_VALUE bar");
    assert_eq!(fs::read_to_string(f3.path()).unwrap(), "bar baz NEW_VALUE");
  }

  #[test]
  fn it_does_not_replace_deselected_matches() {
    let f1 = NamedTempFile::new().unwrap();
    let f2 = NamedTempFile::new().unwrap();
    let f3 = NamedTempFile::new().unwrap();

    let mut items = vec![
      temp_rg_msg(&f1, 0, "foo bar baz", vec![SubMatch::new_text("foo", 0..3)]),
      temp_rg_msg(&f2, 0, "baz foo bar", vec![SubMatch::new_text("foo", 4..7)]),
      temp_rg_msg(
        &f3,
        0,
        "bar baz foo",
        vec![SubMatch::new_text("foo", 8..11)],
      ),
    ];

    items[0].should_replace = false;
    items[1].should_replace = true;
    items[2].should_replace = false;

    perform_replacements(ReplacementCriteria::new("NEW_VALUE", items)).unwrap();
    assert_eq!(fs::read_to_string(f1.path()).unwrap(), "foo bar baz");
    assert_eq!(fs::read_to_string(f2.path()).unwrap(), "baz NEW_VALUE bar");
    assert_eq!(fs::read_to_string(f3.path()).unwrap(), "bar baz foo");
  }

  #[test]
  fn it_performs_multiple_replacements_one_file() {
    let f = NamedTempFile::new().unwrap();
    let item = temp_rg_msg(
      &f,
      0,
      "foo bar baz",
      vec![
        SubMatch::new_text("foo", 0..3),
        SubMatch::new_text("bar", 4..7),
        SubMatch::new_text("baz", 8..11),
      ],
    );

    perform_replacements(ReplacementCriteria::new("NEW_VALUE", vec![item])).unwrap();
    assert_eq!(
      fs::read_to_string(f.path()).unwrap(),
      "NEW_VALUE NEW_VALUE NEW_VALUE"
    );
  }

  #[test]
  fn it_performs_replacements_on_multiple_lines() {
    let mut f = NamedTempFile::new().unwrap();

    f.write_all(b"foo bar baz\n...\nbaz foo bar\n...\nbar baz foo")
      .unwrap();

    let path_string = f.path().to_string_lossy();
    let items = vec![
      Item::new(
        RgMessageBuilder::new(RgMessageKind::Match)
          .with_path_text(&path_string)
          .with_submatches(vec![SubMatch::new_text("foo", 0..3)])
          .with_lines_text("foo bar baz\n")
          .with_offset(0)
          .build(),
      ),
      Item::new(
        RgMessageBuilder::new(RgMessageKind::Match)
          .with_path_text(&path_string)
          .with_submatches(vec![SubMatch::new_text("foo", 4..7)])
          .with_lines_text("baz foo bar\n")
          .with_offset(16)
          .build(),
      ),
      Item::new(
        RgMessageBuilder::new(RgMessageKind::Match)
          .with_path_text(&path_string)
          .with_submatches(vec![SubMatch::new_text("foo", 8..11)])
          .with_lines_text("bar baz foo")
          .with_offset(32)
          .build(),
      ),
    ];

    perform_replacements(ReplacementCriteria::new("NEW_VALUE", items)).unwrap();
    assert_eq!(
      fs::read_to_string(f.path()).unwrap(),
      "NEW_VALUE bar baz\n...\nbaz NEW_VALUE bar\n...\nbar baz NEW_VALUE"
    );
  }

  // TODO: write a similar test for Windows systems
  #[test]
  #[cfg(unix)]
  fn it_performs_replacements_files_with_non_utf8_paths_unix() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    // Here, the values 0x66 and 0x6f correspond to 'f' and 'o'
    // respectively. The value 0x80 is a lone continuation byte, invalid
    // in a UTF-8 sequence.
    let invalid_file_name_bytes = [0x66, 0x6f, 0x80, 0x6f];
    let invalid_file_name = OsStr::from_bytes(&invalid_file_name_bytes[..]);

    let d = tempdir().unwrap();
    let p = PathBuf::from(d.path()).join(invalid_file_name);
    let lines = "hello earth";
    fs::write(&p, lines.as_bytes()).unwrap();

    let item = Item::new(
      RgMessageBuilder::new(RgMessageKind::Match)
        .with_path_base64(base64::encode(p.as_os_str().as_bytes()))
        .with_lines_text(lines.to_string())
        .with_submatches(vec![SubMatch::new_text("o", 4..5)])
        .with_offset(0)
        .build(),
    );

    perform_replacements(ReplacementCriteria::new(" on", vec![item])).unwrap();
    assert_eq!(fs::read_to_string(p).unwrap(), "hell on earth");
  }

  // Encodings

  macro_rules! test_encoding_simple {
    ($name:ident, $src_bytes:expr, $range:expr, $dst_bytes:expr) => {
      #[test]
      fn $name() {
        // Write bytes to temp file.
        let mut f = NamedTempFile::new().unwrap();
        f.write_all($src_bytes).unwrap();

        // Build item match.
        let item = Item::new(
          RgMessageBuilder::new(RgMessageKind::Match)
            .with_path_text(f.path().to_string_lossy())
            .with_lines_text("Ж")
            .with_submatches(vec![SubMatch::new_text("Ж", $range)])
            .with_offset(0)
            .build(),
        );

        // Replace match in file.
        perform_replacements(ReplacementCriteria::new("foo", vec![item])).unwrap();

        // Read file bytes.
        let mut file_bytes = vec![];
        OpenOptions::new()
          .read(true)
          .open(f.path())
          .unwrap()
          .read_to_end(&mut file_bytes)
          .unwrap();

        // Check real bytes are the same as expected bytes.
        assert_eq!(file_bytes, $dst_bytes);
      }
    };
  }

  test_encoding_simple!(encodings_simple_utf8, b"\xD0\x96", 0..2, b"\x66\x6F\x6F");
  test_encoding_simple!(
    encodings_simple_utf16le,
    b"\xFF\xFE\x16\x04",
    0..2,
    b"\xFF\xFE\x66\x00\x6F\x00\x6F\x00"
  );
  test_encoding_simple!(
    encodings_simple_utf16be,
    b"\xFE\xFF\x04\x16",
    0..2,
    b"\xFE\xFF\x00\x66\x00\x6F\x00\x6F"
  );
}
