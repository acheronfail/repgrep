// NOTE: a copy of the `de` mod but with borrows

use std::borrow::Cow;
use std::ffi::OsString;
use std::fmt::{self, Display};
use std::ops::Range;
use std::path::PathBuf;

use anyhow::Result;
use base64_simd::STANDARD as base64;
use serde::{Deserialize, Serialize};

/// A helper to easily select the `RgMessage` kind.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RgMessageKind {
    Begin,
    End,
    Match,
    Context,
    Summary,
}

/// A struct used to deserialise JSON values produced by `ripgrep`.
/// See: https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "data")]
pub enum RgMessage<'a> {
    /// As specified in: [message-begin](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#message-begin).
    Begin {
        #[serde(borrow)]
        path: ArbitraryData<'a>,
    },
    /// As specified in: [message-end](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#message-end).
    End {
        #[serde(borrow)]
        path: ArbitraryData<'a>,
        binary_offset: Option<usize>,
        stats: Stats<'a>,
    },
    /// As specified in: [message-match](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#message-match).
    Match {
        #[serde(borrow)]
        path: ArbitraryData<'a>,
        #[serde(borrow)]
        lines: ArbitraryData<'a>,
        line_number: Option<usize>,
        absolute_offset: usize,
        submatches: Vec<SubMatch<'a>>,
    },
    /// As specified in: [message-context](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#message-context).
    Context {
        #[serde(borrow)]
        path: ArbitraryData<'a>,
        #[serde(borrow)]
        lines: ArbitraryData<'a>,
        line_number: Option<usize>,
        absolute_offset: usize,
        submatches: Vec<SubMatch<'a>>,
    },
    Summary {
        elapsed_total: Duration<'a>,
        stats: Stats<'a>,
    },
}

/// As specified in: [object-arbitrary-data](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#object-arbitrary-data).
/// NOTE: due to how deserialization works with `serde_json`, JSON strings with escape characters in them
/// can't be "borrow"'d, but must be allocated (i.e., `String` not `&str`).
/// See: https://github.com/serde-rs/json/issues/742
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, Hash)]
#[serde(untagged)]
pub enum ArbitraryData<'a> {
    Text {
        #[serde(borrow)]
        text: Cow<'a, str>,
    },
    Base64 {
        #[serde(borrow)]
        bytes: Cow<'a, str>,
    },
}

impl<'a> ArbitraryData<'a> {
    pub fn to_vec(&self) -> Vec<u8> {
        match self {
            ArbitraryData::Text { text } => text.as_bytes().to_vec(),
            ArbitraryData::Base64 { bytes } => base64.decode_to_vec(bytes.as_bytes()).unwrap(),
        }
    }

    /// Converts to an `OsString`.
    #[cfg(unix)]
    pub fn to_os_string(&self) -> Result<OsString> {
        /// Convert Base64 encoded data to an OsString on Unix platforms.
        /// https://doc.rust-lang.org/std/ffi/index.html#on-unix
        use std::os::unix::ffi::OsStringExt;

        Ok(match self {
            ArbitraryData::Text { text } => OsString::from(text.to_string()),
            ArbitraryData::Base64 { .. } => OsString::from_vec(self.to_vec()),
        })
    }

    /// Converts to an `OsString`.
    #[cfg(windows)]
    pub fn to_os_string(&self) -> Result<OsString> {
        /// Convert Base64 encoded data to an OsString on Windows platforms.
        /// https://doc.rust-lang.org/std/ffi/index.html#on-windows
        use std::os::windows::ffi::OsStringExt;

        Ok(match self {
            ArbitraryData::Text { text } => OsString::from(text.to_string()),
            ArbitraryData::Base64 { .. } => {
                // Transmute decoded Base64 bytes as UTF-16 since that's what underlying paths are on Windows.
                let bytes_u16 = safe_transmute::transmute_vec::<u8, u16>(self.to_vec())
                    .or_else(|e| e.copy())?;

                OsString::from_wide(&bytes_u16)
            }
        })
    }

    pub fn to_path_buf(&self) -> Result<PathBuf> {
        self.to_os_string().map(PathBuf::from)
    }

    pub fn lossy_utf8(&self) -> String {
        match self {
            ArbitraryData::Text { text } => text.to_string(),
            ArbitraryData::Base64 { bytes } => {
                String::from_utf8_lossy(base64.decode_to_vec(bytes.as_bytes()).unwrap().as_slice())
                    .to_string()
            }
        }
    }
}

impl<'a> Display for ArbitraryData<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.lossy_utf8())
    }
}

/// As specified in: [object-stats](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#object-stats).
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct Stats<'a> {
    #[serde(borrow)]
    pub elapsed: Duration<'a>,
    pub searches: usize,
    pub searches_with_match: usize,
    pub bytes_searched: usize,
    pub bytes_printed: usize,
    pub matched_lines: usize,
    pub matches: usize,
}

/// As specified in: [object-duration](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#object-duration).
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct Duration<'a> {
    pub secs: usize,
    pub nanos: usize,
    #[serde(borrow)]
    pub human: &'a str,
}

/// Almost as specified in: [object-submatch](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#object-submatch).
/// `match` is deserialized to `text` because a rust reserves match as a keyword.
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename = "submatch")]
pub struct SubMatch<'a> {
    #[serde(rename = "match", borrow)]
    pub text: ArbitraryData<'a>,
    #[serde(flatten)]
    pub range: Range<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn de_bytes() {
        #[derive(Debug, Deserialize, Serialize)]
        struct Foo<'a> {
            data: &'a str,
        }

        let text = "foo\n";
        let data = Foo { data: text };
        let serialised = serde_json::to_string(&data).unwrap();
        assert_eq!(serialised, r#"{"data":"foo\n"}"#);
        dbg!(&data);
        dbg!(data.data.as_bytes());
    }

    #[test]
    fn arbitrary_data_text() {
        let text = "foo\n";
        let data = ArbitraryData::Text { text };
        let ser = serde_json::to_string(&data).unwrap();
        assert_eq!(ser, r#"{"text":"foo\n"}"#);
        let de: ArbitraryData = serde_json::from_str(&ser).unwrap();
        assert_eq!(
            de,
            ArbitraryData::TextOwned {
                text: text.to_string()
            }
        );
    }

    #[test]
    fn arbitrary_data_bytes() {
        let bytes = "text";
        let data = ArbitraryData::Base64 { bytes };
        let ser = serde_json::to_string(&data).unwrap();
        assert_eq!(ser, r#"{"bytes":"text"}"#);
        let de: ArbitraryData = serde_json::from_str(&ser).unwrap();
        assert_eq!(de, data);
    }

    #[test]
    fn submatch() {
        let text = "text";
        let submatch = SubMatch {
            text: ArbitraryData::Text { text },
            range: 0..1,
        };
        let ser = serde_json::to_string(&submatch).unwrap();
        assert_eq!(ser, r#"{"match":{"text":"text"},"start":0,"end":1}"#);
        let de: SubMatch = serde_json::from_str(&ser).unwrap();
        assert_eq!(de, submatch);
    }

    #[test]
    fn rg_message_begin() {
        let text = "foobar";
        let msg = RgMessage::Begin {
            path: ArbitraryData::Text { text },
        };
        let ser = serde_json::to_string(&msg).unwrap();
        assert_eq!(ser, r#"{"type":"begin","data":{"path":{"text":"foobar"}}}"#);
        let de: RgMessage = serde_json::from_str(&ser).unwrap();
        assert_eq!(de, msg);
    }

    #[test]
    fn rg_message_end() {
        let text = "foobar";
        let msg = RgMessage::End {
            binary_offset: None,
            stats: Stats {
                elapsed: Duration {
                    secs: 1,
                    nanos: 1,
                    human: text,
                },
                searches: 1,
                searches_with_match: 1,
                bytes_searched: 1,
                bytes_printed: 1,
                matched_lines: 1,
                matches: 1,
            },
            path: ArbitraryData::Text { text },
        };
        let ser = serde_json::to_string(&msg).unwrap();
        assert_eq!(
            ser,
            r#"{"type":"end","data":{"path":{"text":"foobar"},"binary_offset":null,"stats":{"elapsed":{"secs":1,"nanos":1,"human":"foobar"},"searches":1,"searches_with_match":1,"bytes_searched":1,"bytes_printed":1,"matched_lines":1,"matches":1}}}"#
        );
        let de: RgMessage = serde_json::from_str(&ser).unwrap();
        assert_eq!(de, msg);
    }

    #[test]
    fn rg_message_match() {
        let text = "foo";
        let msg = RgMessage::Match {
            path: ArbitraryData::Text { text },
            lines: ArbitraryData::Text { text },
            line_number: None,
            absolute_offset: 1,
            submatches: vec![],
        };
        let ser = serde_json::to_string(&msg).unwrap();
        assert_eq!(
            ser,
            r#"{"type":"match","data":{"path":{"text":"foo"},"lines":{"text":"foo"},"line_number":null,"absolute_offset":1,"submatches":[]}}"#
        );
        let de: RgMessage = serde_json::from_str(&ser).unwrap();
        assert_eq!(de, msg);
    }

    #[test]
    fn rg_message_context() {
        let text = "foobar";
        let msg = RgMessage::Context {
            path: ArbitraryData::Text { text },
            lines: ArbitraryData::Text { text },
            line_number: None,
            absolute_offset: 1,
            submatches: vec![],
        };
        let ser = serde_json::to_string(&msg).unwrap();
        assert_eq!(
            ser,
            r#"{"type":"context","data":{"path":{"text":"foobar"},"lines":{"text":"foobar"},"line_number":null,"absolute_offset":1,"submatches":[]}}"#
        );
        let de: RgMessage = serde_json::from_str(&ser).unwrap();
        assert_eq!(de, msg);
    }

    #[test]
    fn rg_message_summary() {
        let text = "foobar";
        let msg = RgMessage::Summary {
            elapsed_total: Duration {
                secs: 1,
                nanos: 1,
                human: text,
            },
            stats: Stats {
                elapsed: Duration {
                    secs: 1,
                    nanos: 1,
                    human: text,
                },
                searches: 1,
                searches_with_match: 1,
                bytes_searched: 1,
                bytes_printed: 1,
                matched_lines: 1,
                matches: 1,
            },
        };
        let ser = serde_json::to_string(&msg).unwrap();
        assert_eq!(
            ser,
            r#"{"type":"summary","data":{"elapsed_total":{"secs":1,"nanos":1,"human":"foobar"},"stats":{"elapsed":{"secs":1,"nanos":1,"human":"foobar"},"searches":1,"searches_with_match":1,"bytes_searched":1,"bytes_printed":1,"matched_lines":1,"matches":1}}}"#
        );
        let de: RgMessage = serde_json::from_str(&ser).unwrap();
        assert_eq!(de, msg);
    }
}
