use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};

use crate::model::Item;

#[derive(Debug)]
pub struct ReplacementCriteria {
  pub items: Vec<Item>,
  pub text: String,
}

impl ReplacementCriteria {
  pub fn new(text: impl AsRef<str>, items: Vec<Item>) -> ReplacementCriteria {
    let text = text.as_ref().to_owned();
    ReplacementCriteria { text, items }
  }
}

#[derive(Debug)]
pub struct Replacement {
  path: PathBuf,
  matches: Vec<String>,
}

impl Replacement {
  pub fn new(path: impl AsRef<Path>, matches: &[impl AsRef<str>]) -> Replacement {
    let path = path.as_ref().to_owned();
    let matches = matches.iter().map(|s| s.as_ref().to_owned()).collect();
    Replacement { path, matches }
  }
}

#[derive(Debug)]
pub struct ReplacementResult {
  text: String,
  replacements: Vec<Replacement>,
}

impl ReplacementResult {
  pub fn new(text: impl AsRef<str>) -> ReplacementResult {
    let text = text.as_ref().to_owned();
    ReplacementResult {
      text,
      replacements: vec![],
    }
  }

  pub fn add_replacement(&mut self, path: impl AsRef<Path>, matches: &[impl AsRef<str>]) {
    self.replacements.push(Replacement::new(path, matches));
  }
}

impl Display for ReplacementResult {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    for replacement in &self.replacements {
      if !replacement.matches.is_empty() {
        write!(f, "file: {}", replacement.path.display())?;
        for m in &replacement.matches {
          write!(f, "  replaced {}", m)?;
        }
      }
    }

    write!(f, "Replacement text: {}", self.text)?;
    write!(f, "Total matches replaced: {}", self.replacements.len())?;

    Ok(())
  }
}
