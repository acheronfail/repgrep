use std::fs::{self, OpenOptions};
use std::io::{Read, Write};

use anyhow::Result;

use crate::model::{Item, ItemKind};

// TODO: extensively test this function!
pub fn perform_replacements(items: Vec<Item>, replacement: impl AsRef<str>) -> Result<()> {
  items
    .iter()
    // The only item kind we replace is the Match kind.
    .filter(|item| matches!(item.kind, ItemKind::Match) && item.should_replace)
    // Perform the replacement on each match.
    // TODO: handle non-UTF8 files
    // TODO: better error handling and messaging to the user when any of this fails
    .for_each(|item| {
      let file_path = item.path();

      // Read file to string.
      let mut file_contents = String::new();
      OpenOptions::new()
        .read(true)
        .open(&file_path)
        .unwrap()
        .read_to_string(&mut file_contents)
        .unwrap();

      // Replace matches within the file contents with the given `replacement` string.
      if let Some(submatches) = item.matches() {
        for submatch in submatches.iter().rev() {
          file_contents.replace_range(submatch.range.clone(), replacement.as_ref());
        }
      }

      // Write modified string into a temporary file.
      let temp_file_path = &file_path.with_extension("rgr");
      OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&temp_file_path)
        .unwrap()
        .write_all(file_contents.as_bytes())
        .unwrap();

      // Overwrite the original file with the patched temp file.
      fs::rename(temp_file_path, file_path).unwrap();
    });

  Ok(())
}

#[cfg(test)]
mod tests {
  use std::fs;
  use std::io::Write;
  use std::path::PathBuf;

  use tempfile::{tempdir, NamedTempFile};

  use crate::model::Item;
  use crate::replace::perform_replacements;
  use crate::rg::de::test_utilities::{RgMessageBuilder, RgMessageKind};
  use crate::rg::de::SubMatch;

  fn tempitem(mut f: &NamedTempFile, lines: impl AsRef<str>, submatches: Vec<SubMatch>) -> Item {
    f.write_all(lines.as_ref().as_bytes()).unwrap();

    Item::new(
      RgMessageBuilder::new(RgMessageKind::Match)
        .with_path_text(f.path().to_string_lossy().to_string())
        .with_lines_text(lines.as_ref().to_string())
        .with_submatches(submatches)
        .with_offset(0) // unused at the moment
        .build(),
    )
  }

  #[test]
  fn it_performs_replacements_in_separate_files() {
    let f1 = NamedTempFile::new().unwrap();
    let f2 = NamedTempFile::new().unwrap();
    let f3 = NamedTempFile::new().unwrap();

    let items = vec![
      tempitem(&f1, "foo bar baz", vec![SubMatch::new_text("foo", 0..3)]),
      tempitem(&f2, "baz foo bar", vec![SubMatch::new_text("foo", 4..7)]),
      tempitem(&f3, "bar baz foo", vec![SubMatch::new_text("foo", 8..11)]),
    ];

    perform_replacements(items, "ZAP").unwrap();
    assert_eq!(fs::read_to_string(f1.path()).unwrap(), "ZAP bar baz");
    assert_eq!(fs::read_to_string(f2.path()).unwrap(), "baz ZAP bar");
    assert_eq!(fs::read_to_string(f3.path()).unwrap(), "bar baz ZAP");
  }

  #[test]
  fn it_performs_multiple_replacements_one_file() {
    let f = NamedTempFile::new().unwrap();
    let item = tempitem(
      &f,
      "foo bar baz",
      vec![
        SubMatch::new_text("foo", 0..3),
        SubMatch::new_text("bar", 4..7),
        SubMatch::new_text("baz", 8..11),
      ],
    );

    perform_replacements(vec![item], "ZAP").unwrap();
    assert_eq!(fs::read_to_string(f.path()).unwrap(), "ZAP ZAP ZAP");
  }

  #[test]
  fn it_does_not_replace_deselected_matches() {
    let f1 = NamedTempFile::new().unwrap();
    let f2 = NamedTempFile::new().unwrap();
    let f3 = NamedTempFile::new().unwrap();

    let mut items = vec![
      tempitem(&f1, "foo bar baz", vec![SubMatch::new_text("foo", 0..3)]),
      tempitem(&f2, "baz foo bar", vec![SubMatch::new_text("foo", 4..7)]),
      tempitem(&f3, "bar baz foo", vec![SubMatch::new_text("foo", 8..11)]),
    ];

    items[0].should_replace = false;
    items[1].should_replace = true;
    items[2].should_replace = false;

    perform_replacements(items, "ZAP").unwrap();
    assert_eq!(fs::read_to_string(f1.path()).unwrap(), "foo bar baz");
    assert_eq!(fs::read_to_string(f2.path()).unwrap(), "baz ZAP bar");
    assert_eq!(fs::read_to_string(f3.path()).unwrap(), "bar baz foo");
  }

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
        .with_offset(0) // unused at the moment
        .build(),
    );

    println!("{:#?}", &d);
    println!("{:#?}", &item);
    println!("{}", fs::read_to_string(&p).unwrap());

    perform_replacements(vec![item], " on").unwrap();
    assert_eq!(fs::read_to_string(p).unwrap(), "hell on earth");
  }
}
