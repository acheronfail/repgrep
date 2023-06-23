use std::collections::hash_map::Entry;
use std::collections::HashMap;

use regex::bytes::Regex;

use crate::rg::de::{ArbitraryData, RgMessageKind};
use crate::ui::line::Item;

#[derive(Debug)]
pub struct ReplacementCriteria {
    pub capture_pattern: Option<Regex>,
    pub items: Vec<Item>,
    pub user_replacement: Vec<u8>,
    pub encoding: Option<String>,
}

impl ReplacementCriteria {
    pub fn new<S: AsRef<str>>(
        capture_pattern: Option<Regex>,
        user_replacement: S,
        items: Vec<Item>,
    ) -> ReplacementCriteria {
        ReplacementCriteria {
            capture_pattern,
            user_replacement: user_replacement.as_ref().as_bytes().to_vec(),
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
