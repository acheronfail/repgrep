// NOTE: Originally adapted from the `grep_json_deserialize` crate.
// See: https://github.com/Avi-D-coder/grep_json_deserialize/blob/master/src/lib.rs

use std::ops::Range;

use serde::{Deserialize, Serialize};

/// A struct used to deserialise JSON values produced by `ripgrep`.
/// See: https://docs.rs/grep-printer/0.1.5/grep_printer/struct.JSON.html
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "data")]
pub enum RgMessageType {
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
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum ArbitraryData {
  Text { text: String },
  Base64 { bytes: String },
}

impl ArbitraryData {
  pub fn lossy_utf8(&self) -> String {
    match self {
      ArbitraryData::Text { text } => text.to_owned(),
      ArbitraryData::Base64 { bytes } => {
        String::from_utf8_lossy(base64::decode(bytes).unwrap().as_slice()).to_string()
      }
    }
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

  use crate::rg::de::{ArbitraryData::*, RgMessageType::*, *};

  #[test]
  fn arbitrarydata() {
    let json = r#"{"text":"/home/andrew/sherlock"}"#;
    assert_eq!(
      Text {
        text: "/home/andrew/sherlock".to_owned()
      },
      serde_json::from_str(json).unwrap()
    )
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
          text: "but Doctor Watson has to have it taken out for him and dusted,\n".to_owned()
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
          text: "can extract a clew from a wisp of straw or a flake of cigar ash;\n".to_owned()
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
