use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::rg::de::{ArbitraryData, RgMessageKind};
use crate::ui::line::Item;

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

    pub fn as_map(&self) -> HashMap<&ArbitraryData, Vec<&Item>> {
        self.items
            .iter()
            // The only item kind we replace is the Match kind.
            .filter(|item| matches!(item.kind, RgMessageKind::Match))
            // Collect into a map of paths -> matches.
            .fold(HashMap::new(), |mut map, item| {
                match map.entry(item.path().unwrap()) {
                    Entry::Occupied(e) => e.into_mut().push(item),
                    Entry::Vacant(e) => {
                        e.insert(vec![item]);
                    }
                }

                map
            })
    }
}
