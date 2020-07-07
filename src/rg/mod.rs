pub mod de;
pub mod exec;

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
