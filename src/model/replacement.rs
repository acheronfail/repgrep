use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};

use crate::model::Item;

#[derive(Debug)]
pub struct ReplacementCriteria {
  pub items: Vec<Item>,
  pub text: String,
  pub encoding: Option<String>,
}

impl ReplacementCriteria {
  pub fn new<S: AsRef<str>>(text: S, items: Vec<Item>) -> ReplacementCriteria {
    let text = text.as_ref().to_owned();
    ReplacementCriteria {
      text,
      items,
      encoding: None,
    }
  }

  pub fn set_encoding(&mut self, encoding: impl AsRef<str>) {
    self.encoding = Some(encoding.as_ref().to_owned());
  }
}

#[derive(Debug)]
pub struct ReplacementResult {
  pub text: String,
  /// Map of (Path, DetectedEncoding) -> [List, Of, Matches, Replaced]
  pub replacements: HashMap<(PathBuf, String), Vec<String>>,
}

impl ReplacementResult {
  pub fn new(text: impl AsRef<str>) -> ReplacementResult {
    let text = text.as_ref().to_owned();
    ReplacementResult {
      text,
      replacements: HashMap::new(),
    }
  }

  pub fn add_replacement<P, S>(&mut self, path: P, matches: &[S], detected_encoding: S)
  where
    P: AsRef<Path>,
    S: AsRef<str>,
  {
    let path = path.as_ref().to_owned();
    let detected_encoding = detected_encoding.as_ref().to_owned();
    let mut matches = matches.iter().map(|s| s.as_ref().to_owned()).collect();

    self
      .replacements
      .entry((path, detected_encoding))
      .and_modify(|v| (*v).append(&mut matches))
      .or_insert_with(|| matches);
  }
}

impl Display for ReplacementResult {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    let mut total_replacements = 0;
    for ((path, encoding), replacements) in &self.replacements {
      if !replacements.is_empty() {
        writeln!(f, "file: {} <{}>", path.display(), encoding)?;
        for r in replacements {
          writeln!(f, "  replaced: {}", r)?;
          total_replacements += 1;
        }
      }
    }

    writeln!(f)?;
    writeln!(f, "Replacement text: {}", self.text)?;
    writeln!(f, "Total matches replaced: {}", total_replacements)?;

    Ok(())
  }
}
