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
