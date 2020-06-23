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
  detected_encoding: String,
}

impl Replacement {
  pub fn new<P, S>(path: P, matches: &[S], detected_encoding: S) -> Replacement
  where
    P: AsRef<Path>,
    S: AsRef<str>,
  {
    let path = path.as_ref().to_owned();
    let matches = matches.iter().map(|s| s.as_ref().to_owned()).collect();
    let detected_encoding = detected_encoding.as_ref().to_owned();
    Replacement {
      path,
      matches,
      detected_encoding,
    }
  }
}

impl Display for Replacement {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    writeln!(
      f,
      "file: {} <{}>",
      self.path.display(),
      self.detected_encoding
    )?;

    if !self.matches.is_empty() {
      for m in &self.matches {
        writeln!(f, "  replaced {}", m)?;
      }
    } else {
      writeln!(f, "  no matches")?;
    }

    Ok(())
  }
}

#[derive(Debug)]
pub struct ReplacementResult {
  pub text: String,
  pub replacements: Vec<Replacement>,
}

impl ReplacementResult {
  pub fn new(text: impl AsRef<str>) -> ReplacementResult {
    let text = text.as_ref().to_owned();
    ReplacementResult {
      text,
      replacements: vec![],
    }
  }

  pub fn add_replacement<P, S>(&mut self, path: P, matches: &[S], detected_encoding: S)
  where
    P: AsRef<Path>,
    S: AsRef<str>,
  {
    self
      .replacements
      .push(Replacement::new(path, matches, detected_encoding));
  }
}

impl Display for ReplacementResult {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    for replacement in &self.replacements {
      writeln!(f, "{}", replacement)?;
    }

    writeln!(f, "Replacement text: {}", self.text)?;
    writeln!(f, "Total matches replaced: {}", self.replacements.len())?;

    Ok(())
  }
}
