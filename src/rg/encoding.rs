use std::convert::From;
use std::fmt::{self, Debug};

use encoding::label::encoding_from_whatwg_label;
use encoding::EncodingRef;

/// A small wrapper to help describe the encoding that we think ripgrep will use.
pub enum RgEncoding {
    /// A valid encoding was passed and this is the reference to its encoder.
    Some(EncodingRef),
    /// Either the option wasn't passed, or it wasn't a valid encoding.
    None,
}

impl RgEncoding {
    /// Returns an `EncodingRef` for this `RgEncoding`, if any exists.
    pub fn encoder(&self) -> Option<EncodingRef> {
        match &self {
            RgEncoding::Some(enc) => Some(*enc),
            RgEncoding::None => None,
        }
    }
}

impl Debug for RgEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RgEncoding::None => write!(f, "RgEncoding::None"),
            RgEncoding::Some(encoding) => write!(f, "RgEncoding::Some({})", encoding.name()),
        }
    }
}

impl From<&str> for RgEncoding {
    fn from(s: &str) -> Self {
        encoding_from_whatwg_label(s).map_or_else(|| RgEncoding::None, |e| RgEncoding::Some(e))
    }
}

impl From<&Option<String>> for RgEncoding {
    fn from(input: &Option<String>) -> Self {
        match input {
            Some(label) => Self::from(label.as_str()),
            None => RgEncoding::None,
        }
    }
}
