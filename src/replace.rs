use anyhow::Result;

use crate::model::{Item, ItemKind};

pub fn perform_replacements(items: Vec<Item>) -> Result<()> {
  items
    .iter()
    .filter(|item| matches!(item.kind, ItemKind::Match) && item.should_replace)
    .for_each(|item| {
      // TODO: perform replacements
    });
  Ok(())
}
