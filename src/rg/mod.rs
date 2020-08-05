pub mod de;
pub mod exec;
pub mod read;

use std::convert::From;

use encoding::label::encoding_from_whatwg_label;
use encoding::EncodingRef;

/// A small wrapper to help describe the encoding that we think ripgrep will use.
pub enum RgEncoding {
    /// A valid encoding was passed and this is the reference to its encoder.
    Some(EncodingRef),
    /// The user explicitly passed "none".
    NoneExplicit,
    /// Either the option wasn't passed, or it wasn't a valid encoding.
    None,
}

impl RgEncoding {
    /// Returns an `EncodingRef` for this `RgEncoding`, if any exists.
    pub fn encoder(&self) -> Option<EncodingRef> {
        match &self {
            RgEncoding::Some(enc) => Some(*enc),
            _ => None,
        }
    }
}

impl From<&str> for RgEncoding {
    fn from(s: &str) -> Self {
        if s == "none" {
            RgEncoding::NoneExplicit
        } else {
            encoding_from_whatwg_label(s).map_or_else(|| RgEncoding::None, |e| RgEncoding::Some(e))
        }
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
