use std::collections::HashMap;
use std::collections::hash_map::Entry;

use regex::bytes::Regex;

use crate::rg::de::{ArbitraryData, RgMessageKind};
use crate::ui::line::Item;

/// Expand capture references in a replacement, falling back to the complete
/// ripgrep match as capture group 0 when no capture pattern is available.
pub fn expand_replacement(
    capture_pattern: Option<&Regex>,
    matched_bytes: &[u8],
    user_replacement: &[u8],
    dst: &mut Vec<u8>,
) {
    if let Some(captures) = capture_pattern.and_then(|re| re.captures(matched_bytes)) {
        captures.expand(user_replacement, dst);
        return;
    }

    let mut remaining = user_replacement;
    while let Some(start) = remaining.iter().position(|&b| b == b'$') {
        dst.extend_from_slice(&remaining[..start]);
        remaining = &remaining[start..];

        if remaining.starts_with(b"$$") {
            dst.push(b'$');
            remaining = &remaining[2..];
            continue;
        }

        let (capture, end) = if remaining.starts_with(b"${") {
            match remaining[2..].iter().position(|&b| b == b'}') {
                Some(end) => (&remaining[2..end + 2], end + 3),
                None => {
                    dst.push(b'$');
                    remaining = &remaining[1..];
                    continue;
                }
            }
        } else {
            let end = remaining[1..]
                .iter()
                .position(|&b| !b.is_ascii_alphanumeric() && b != b'_')
                .map_or(remaining.len(), |end| end + 1);
            (&remaining[1..end], end)
        };

        if capture.is_empty()
            || capture
                .iter()
                .any(|&b| !b.is_ascii_alphanumeric() && b != b'_')
        {
            dst.push(b'$');
            remaining = &remaining[1..];
            continue;
        }
        if capture.iter().all(|&b| b == b'0') {
            dst.extend_from_slice(matched_bytes);
        }
        remaining = &remaining[end..];
    }
    dst.extend_from_slice(remaining);
}

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
