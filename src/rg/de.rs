// NOTE: Originally adapted from the `grep_json_deserialize` crate.
// See: https://github.com/Avi-D-coder/grep_json_deserialize/blob/master/src/lib.rs

use std::ffi::OsString;
use std::fmt::{self, Display};
use std::ops::Range;
use std::path::PathBuf;

use anyhow::Result;
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
pub enum RgMessage {
    /// As specified in: [message-begin](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#message-begin).
    Begin { path: ArbitraryData },
    /// As specified in: [message-end](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#message-end).
    End {
        path: ArbitraryData,
        binary_offset: Option<usize>,
        stats: Stats,
    },
    /// As specified in: [message-match](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#message-match).
    Match {
        path: ArbitraryData,
        lines: ArbitraryData,
        line_number: Option<usize>,
        absolute_offset: usize,
        submatches: Vec<SubMatch>,
    },
    /// As specified in: [message-context](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#message-context).
    Context {
        path: ArbitraryData,
        lines: ArbitraryData,
        line_number: Option<usize>,
        absolute_offset: usize,
        submatches: Vec<SubMatch>,
    },
    Summary {
        elapsed_total: Duration,
        stats: Stats,
    },
}

/// As specified in: [object-arbitrary-data](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#object-arbitrary-data).
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, Hash)]
#[serde(untagged)]
pub enum ArbitraryData {
    Text { text: String },
    Base64 { bytes: String },
}

impl ArbitraryData {
    pub fn to_vec(&self) -> Vec<u8> {
        match self {
            ArbitraryData::Text { text } => text.as_bytes().to_vec(),
            ArbitraryData::Base64 { bytes } => base64::decode(bytes).unwrap(),
        }
    }

    /// Converts to an `OsString`.
    #[cfg(unix)]
    pub fn to_os_string(&self) -> Result<OsString> {
        /// Convert Base64 encoded data to an OsString on Unix platforms.
        /// https://doc.rust-lang.org/std/ffi/index.html#on-unix
        use std::os::unix::ffi::OsStringExt;

        Ok(match self {
            ArbitraryData::Text { text } => OsString::from(text),
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
            ArbitraryData::Text { text } => OsString::from(text),
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
            ArbitraryData::Text { text } => text.to_owned(),
            ArbitraryData::Base64 { bytes } => {
                String::from_utf8_lossy(base64::decode(bytes).unwrap().as_slice()).to_string()
            }
        }
    }
}

impl Display for ArbitraryData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.lossy_utf8())
    }
}

/// As specified in: [object-stats](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#object-stats).
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct Stats {
    pub elapsed: Duration,
    pub searches: usize,
    pub searches_with_match: usize,
    pub bytes_searched: usize,
    pub bytes_printed: usize,
    pub matched_lines: usize,
    pub matches: usize,
}

/// As specified in: [object-duration](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#object-duration).
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct Duration {
    pub secs: usize,
    pub nanos: usize,
    pub human: String,
}

/// Almost as specified in: [object-submatch](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#object-submatch).
/// `match` is deserialized to `text` because a rust reserves match as a keyword.
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename = "submatch")]
pub struct SubMatch {
    #[serde(rename = "match")]
    pub text: ArbitraryData,
    #[serde(flatten)]
    pub range: Range<usize>,
}

#[cfg(test)]
mod tests {
    // tests based on [`grep_printer` example output](https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html#example)

    use pretty_assertions::assert_eq;

    use crate::rg::de::{ArbitraryData::*, RgMessage::*, *};

    #[test]
    fn arbitrary_data() {
        let json = r#"{"text":"/home/andrew/sherlock"}"#;
        assert_eq!(
            Text {
                text: "/home/andrew/sherlock".to_owned()
            },
            serde_json::from_str(json).unwrap()
        )
    }

    #[cfg(unix)]
    #[test]
    fn arbitrary_data_to_os_string_unix() {
        use std::os::unix::ffi::OsStringExt;

        // Here, the values 0x66 and 0x6f correspond to 'f' and 'o'
        // respectively. The value 0x80 is a lone continuation byte, invalid
        // in a UTF-8 sequence.
        let invalid_utf8 = vec![0x66, 0x6f, 0x80, 0x6f];
        let invalid_utf8_os_string = OsString::from_vec(invalid_utf8.clone());

        let data = ArbitraryData::Base64 {
            bytes: base64::encode(&invalid_utf8[..]),
        };

        assert_eq!(data.to_os_string().unwrap(), invalid_utf8_os_string);
    }

    #[cfg(windows)]
    #[test]
    fn arbitrary_data_to_os_string_windows() {
        use std::os::windows::ffi::OsStringExt;

        // Here, the values 0x66 and 0x6f correspond to 'f' and 'o'
        // respectively. The value 0x80 is a lone continuation byte, invalid
        // in a UTF-8 sequence.
        // They're encoded as u16 bytes here since that's what the should be on Windows. (I think?)
        let invalid_utf8_wide = vec![0x0066, 0x006f, 0x0080, 0x006f];
        let invalid_utf8_os_string = OsString::from_wide(&invalid_utf8_wide[..]);

        let bytes_as_u8 = safe_transmute::transmute_to_bytes(&invalid_utf8_wide[..]);
        let data = ArbitraryData::Base64 {
            bytes: base64::encode(&bytes_as_u8[..]),
        };

        assert_eq!(data.to_os_string().unwrap(), invalid_utf8_os_string);
    }

    #[test]
    fn begin_deserialize() {
        let json = r#"{"type":"begin","data":{"path":{"text":"/home/andrew/sherlock"}}}"#;
        assert_eq!(
            Begin {
                path: Text {
                    text: "/home/andrew/sherlock".to_owned()
                }
            },
            serde_json::from_str(json).unwrap()
        );
    }

    #[test]
    fn end_deserialize() {
        let json = r#"{"type":"end","data":{"path":{"text":"/home/andrew/sherlock"},"binary_offset":null,"stats":{"elapsed":{"secs":0,"nanos":36296,"human":"0.0000s"},"searches":1,"searches_with_match":1,"bytes_searched":367,"bytes_printed":1151,"matched_lines":2,"matches":2}}}"#;
        assert_eq!(
            End {
                path: Text {
                    text: "/home/andrew/sherlock".to_owned()
                },
                binary_offset: None,
                stats: Stats {
                    elapsed: Duration {
                        secs: 0,
                        nanos: 36296,
                        human: "0.0000s".to_owned()
                    },
                    searches: 1,
                    searches_with_match: 1,
                    bytes_searched: 367,
                    bytes_printed: 1151,
                    matched_lines: 2,
                    matches: 2
                }
            },
            serde_json::from_str(json).unwrap()
        );
    }

    #[test]
    fn match_deserialize() {
        let json = r#"{"type":"match","data":{"path":{"text":"/home/andrew/sherlock"},"lines":{"text":"but Doctor Watson has to have it taken out for him and dusted,\n"},"line_number":5,"absolute_offset":258,"submatches":[{"match":{"text":"Watson"},"start":11,"end":17}]}}"#;
        assert_eq!(
            Match {
                path: Text {
                    text: "/home/andrew/sherlock".to_owned()
                },
                lines: Text {
                    text: "but Doctor Watson has to have it taken out for him and dusted,\n"
                        .to_owned()
                },
                line_number: Some(5),
                absolute_offset: 258,
                submatches: vec![SubMatch {
                    text: Text {
                        text: "Watson".to_owned()
                    },
                    range: (11..17)
                }],
            },
            serde_json::from_str(json).unwrap()
        )
    }

    #[test]
    fn content_deserialize() {
        let json = r#"{"type":"context","data":{"path":{"text":"/home/andrew/sherlock"},"lines":{"text":"can extract a clew from a wisp of straw or a flake of cigar ash;\n"},"line_number":4,"absolute_offset":193,"submatches":[]}}"#;
        assert_eq!(
            Context {
                path: Text {
                    text: "/home/andrew/sherlock".to_owned()
                },
                lines: Text {
                    text: "can extract a clew from a wisp of straw or a flake of cigar ash;\n"
                        .to_owned()
                },
                line_number: Some(4),
                absolute_offset: 193,
                submatches: vec![],
            },
            serde_json::from_str(json).unwrap()
        )
    }

    #[test]
    fn summary_deserialize() {
        let json = r#"{"data":{"elapsed_total":{"human":"0.099726s","nanos":99726344,"secs":0},"stats":{"bytes_printed":4106,"bytes_searched":5860,"elapsed":{"human":"0.000047s","nanos":46800,"secs":0},"matched_lines":3,"matches":3,"searches":1,"searches_with_match":1}},"type":"summary"}"#;
        assert_eq!(
            Summary {
                elapsed_total: Duration {
                    human: "0.099726s".to_string(),
                    nanos: 99_726_344,
                    secs: 0
                },
                stats: Stats {
                    bytes_printed: 4106,
                    bytes_searched: 5860,
                    elapsed: Duration {
                        human: "0.000047s".to_owned(),
                        nanos: 46800,
                        secs: 0,
                    },
                    matched_lines: 3,
                    matches: 3,
                    searches: 1,
                    searches_with_match: 1
                }
            },
            serde_json::from_str(json).unwrap()
        )
    }
}

/// Utilities for tests.
#[cfg(test)]
#[allow(dead_code)]
pub mod test_utilities {
    use crate::rg::de::*;

    pub const RG_JSON_BEGIN: &str =
        r#"{"type":"begin","data":{"path":{"text":"src/model/item.rs"}}}"#;
    pub const RG_JSON_MATCH: &str = r#"{"type":"match","data":{"path":{"text":"src/model/item.rs"},"lines":{"text":"    Item::new(rg_msg)\n"},"line_number":197,"absolute_offset":5522,"submatches":[{"match":{"text":"Item"},"start":4,"end":8},{"match":{"text":"rg_msg"},"start":14,"end":20}]}}"#;
    pub const RG_JSON_CONTEXT: &str = r#"{"type":"context","data":{"path":{"text":"src/model/item.rs"},"lines":{"text":"  }\n"},"line_number":198,"absolute_offset":5544,"submatches":[]}}"#;
    pub const RG_JSON_END: &str = r#"{"type":"end","data":{"path":{"text":"src/model/item.rs"},"binary_offset":null,"stats":{"elapsed":{"secs":0,"nanos":97924,"human":"0.000098s"},"searches":1,"searches_with_match":1,"bytes_searched":5956,"bytes_printed":674,"matched_lines":2,"matches":2}}}"#;
    pub const RG_JSON_SUMMARY: &str = r#"{"data":{"elapsed_total":{"human":"0.013911s","nanos":13911027,"secs":0},"stats":{"bytes_printed":3248,"bytes_searched":18789,"elapsed":{"human":"0.000260s","nanos":260276,"secs":0},"matched_lines":10,"matches":10,"searches":2,"searches_with_match":2}},"type":"summary"}"#;

    pub const RG_JSON_MATCH_MULTILINE: &str = r#"{"type":"match","data":{"path":{"text":"./foo/baz"},"lines":{"text":"baz 1\n22\n333 bar 4444\n"},"line_number":3,"absolute_offset":16,"submatches":[{"match":{"text":"1\n22\n333"},"start":4,"end":12},{"match":{"text":"4444"},"start":17,"end":21}]}}"#;

    pub const RG_JSON_MATCH_LINE_WRAP: &str = r#"{"type":"match","data":{"path":{"text":"./foo/baz"},"lines":{"text":"123456789!123456789@123456789#123456789$123456789%123456789^123456789&123456789*123456789(123456789_one_hundred_characters_wowzers\n"},"line_number":3,"absolute_offset":16,"submatches":[{"match":{"text":"one_hundred"},"start":100,"end":111}]}}"#;
    pub const RG_JSON_MATCH_LINE_WRAP_MULTI: &str = r#"{"type":"match","data":{"path":{"text":"./foo/baz"},"lines":{"text":"foo foo foo foo foo bar foo foo foo foo foo bar foo foo foo foo foo bar foo foo foo foo foo bar foo foo foo foo foo bar foo foo foo foo foo bar foo foo foo foo foo bar\n"},"line_number":1,"absolute_offset":0,"submatches":[{"match":{"text":"bar"},"start":20,"end":23},{"match":{"text":"bar"},"start":44,"end":47},{"match":{"text":"bar"},"start":68,"end":71},{"match":{"text":"bar"},"start":92,"end":95},{"match":{"text":"bar"},"start":116,"end":119},{"match":{"text":"bar"},"start":140,"end":143},{"match":{"text":"bar"},"start":164,"end":167}]}}"#;
    pub const RG_JSON_CONTEXT_LINE_WRAP: &str = r#"{"type":"context","data":{"path":{"text":"./foo/baz"},"lines":{"text":"123456789!123456789@123456789#123456789$123456789%123456789^123456789&123456789*123456789(123456789_a_context_line\n"},"line_number":4,"absolute_offset":131,"submatches":[]}}"#;

    pub const RG_B64_JSON_BEGIN: &str =
        r#"{"type":"begin","data":{"path":{"bytes":"Li9hL2Zv/28="}}}"#;
    pub const RG_B64_JSON_MATCH: &str = r#"{"type":"match","data":{"path":{"text":"src/model/item.rs"},"lines":{"bytes":"ICAgIP9JdGVtOjr/bmV3KHJnX21zZykK"},"line_number":197,"absolute_offset":5522,"submatches":[{"match":{"text":"Item"},"start":5,"end":9},{"match":{"text":"rg_msg"},"start":16,"end":22}]}}"#;
    pub const RG_B64_JSON_CONTEXT: &str = r#"{"type":"context","data":{"path":{"text":"src/model/item.rs"},"lines":{"bytes":"ICD/fQo="},"line_number":198,"absolute_offset":5544,"submatches":[]}}"#;
    pub const RG_B64_JSON_END: &str = r#"{"type":"end","data":{"path":{"bytes":"Li9hL2Zv/28="},"binary_offset":null,"stats":{"elapsed":{"secs":0,"nanos":64302,"human":"0.000064s"},"searches":1,"searches_with_match":1,"bytes_searched":4,"bytes_printed":235,"matched_lines":1,"matches":1}}}"#;

    impl RgMessage {
        pub fn from_str(raw_json: &str) -> RgMessage {
            serde_json::from_str::<RgMessage>(raw_json).unwrap()
        }
    }

    impl ArbitraryData {
        pub fn new_with_text(text: String) -> ArbitraryData {
            ArbitraryData::Text { text }
        }

        pub fn new_with_base64(bytes: String) -> ArbitraryData {
            ArbitraryData::Base64 { bytes }
        }
    }

    impl SubMatch {
        pub fn new_text(text: impl AsRef<str>, range: Range<usize>) -> SubMatch {
            SubMatch {
                text: ArbitraryData::new_with_text(text.as_ref().to_owned()),
                range,
            }
        }
        pub fn new_base64(base64: impl AsRef<str>, range: Range<usize>) -> SubMatch {
            SubMatch {
                text: ArbitraryData::new_with_base64(base64.as_ref().to_owned()),
                range,
            }
        }
    }

    impl Duration {
        pub fn new() -> Duration {
            Duration {
                human: String::from("0"),
                nanos: 0,
                secs: 0,
            }
        }
    }

    impl Stats {
        pub fn new() -> Stats {
            Stats {
                bytes_printed: 0,
                bytes_searched: 0,
                matched_lines: 0,
                matches: 0,
                searches: 0,
                searches_with_match: 0,
                elapsed: Duration::new(),
            }
        }
    }

    /// A builder to help construct `RgMessage` structs during tests.
    pub struct RgMessageBuilder {
        kind: RgMessageKind,
        path: Option<ArbitraryData>,
        offset: Option<usize>,
        lines: Option<ArbitraryData>,
        line_number: Option<usize>,
        elapsed_total: Option<Duration>,
        stats: Option<Stats>,
        submatches: Vec<SubMatch>,
    }

    impl RgMessageBuilder {
        pub fn new(kind: RgMessageKind) -> RgMessageBuilder {
            RgMessageBuilder {
                kind,
                path: None,
                offset: None,
                lines: None,
                line_number: None,
                stats: None,
                elapsed_total: None,
                submatches: vec![],
            }
        }

        pub fn with_offset(mut self, offset: usize) -> Self {
            self.offset = Some(offset);
            self
        }

        pub fn with_line_number(mut self, line_number: usize) -> Self {
            self.line_number = Some(line_number);
            self
        }

        pub fn with_path_text(mut self, path: impl AsRef<str>) -> Self {
            self.path = Some(ArbitraryData::new_with_text(path.as_ref().to_owned()));
            self
        }

        pub fn with_lines_text(mut self, lines: impl AsRef<str>) -> Self {
            self.lines = Some(ArbitraryData::new_with_text(lines.as_ref().to_owned()));
            self
        }

        pub fn with_path_base64(mut self, path: impl AsRef<str>) -> Self {
            self.path = Some(ArbitraryData::new_with_base64(path.as_ref().to_owned()));
            self
        }

        pub fn with_lines_base64(mut self, lines: impl AsRef<str>) -> Self {
            self.lines = Some(ArbitraryData::new_with_base64(lines.as_ref().to_owned()));
            self
        }

        pub fn with_submatches(mut self, submatches: Vec<SubMatch>) -> Self {
            self.submatches = submatches;
            self
        }

        pub fn with_elapsed_total(mut self, elapsed_total: Duration) -> Self {
            self.elapsed_total = Some(elapsed_total);
            self
        }

        pub fn with_stats(mut self, stats: Stats) -> Self {
            self.stats = Some(stats);
            self
        }

        pub fn build(self) -> RgMessage {
            match self.kind {
                RgMessageKind::Begin => RgMessage::Begin {
                    path: self.path.unwrap(),
                },
                RgMessageKind::End => RgMessage::End {
                    path: self.path.unwrap(),
                    binary_offset: self.offset,
                    stats: self.stats.unwrap(),
                },
                RgMessageKind::Match => RgMessage::Match {
                    path: self.path.unwrap(),
                    absolute_offset: self.offset.unwrap(),
                    line_number: self.line_number,
                    lines: self.lines.unwrap(),
                    submatches: self.submatches,
                },
                RgMessageKind::Context => RgMessage::Context {
                    path: self.path.unwrap(),
                    absolute_offset: self.offset.unwrap(),
                    line_number: self.line_number,
                    lines: self.lines.unwrap(),
                    submatches: self.submatches,
                },
                RgMessageKind::Summary => RgMessage::Summary {
                    elapsed_total: self.elapsed_total.unwrap(),
                    stats: self.stats.unwrap(),
                },
            }
        }
    }
}
