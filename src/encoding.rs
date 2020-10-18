use chardet::charset2encoding;
use encoding::label::encoding_from_whatwg_label;
use encoding::EncodingRef;

use crate::rg::RgEncoding;

/// Returns a tuple of a BOM (if one exists) and an encoding.
pub fn get_encoder(bytes: &[u8], rg_encoding: &RgEncoding) -> (Option<Bom>, EncodingRef) {
    // Check if this file has a BOM (Byte Order Mark).
    let bom = Bom::from_slice(&bytes);

    // Try to detect the encoding of the file.
    let encoder = bom
        // if we found a BOM then use that encoding
        .map(|b| {
            let encoder = b.encoder();
            log::debug!("Found BOM: {:?}, using encoder: {}", b, encoder.name());
            encoder
        })
        // otherwise if the user passed an encoding use that
        .or_else(|| {
            let encoder = rg_encoding.encoder();
            if encoder.is_some() {
                log::debug!(
                    "Found user encoding: {:?}, using encoder: {}",
                    rg_encoding,
                    encoder.unwrap().name()
                );
            }

            encoder
        })
        // nothing so far, try detecting the encoding
        .or_else(|| {
            let (encoding, confidence, _) = chardet::detect(&bytes);
            log::debug!(
                "Attempting to detect encoding - cncoding: {}, Confidence: {}",
                encoding,
                confidence
            );

            // TODO: be able to adjust chardet confidence here
            if confidence > 0.80 {
                // If we pass "ascii" to `encoding_from_whatwg_label` then it will default to using the "windows-1252"
                // encoding. However, this may be confusing as most users are more familiar with ASCII encodings and may
                // be unaware that "windows-1252" is an ASCII compatible encoding.
                if encoding == "ascii" {
                    Some(encoding::all::ASCII)
                } else {
                    encoding_from_whatwg_label(charset2encoding(&encoding))
                }
            } else {
                None
            }
        })
        // if all else fails, assume ASCII
        .unwrap_or_else(|| {
            log::debug!(
                "Failed to detect encoding or confidence was too low, falling back to UTF-8"
            );
            encoding::all::UTF_8
        });

    (bom, encoder)
}

/// A small wrapper to help with BOM (Byte Order Mark) detection.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Bom {
    Utf8,
    Utf16be,
    Utf16le,
}

impl Bom {
    const BOM_UTF8: [u8; 3] = [0xEF, 0xBB, 0xBF];
    const BOM_UTF16BE: [u8; 2] = [0xFE, 0xFF];
    const BOM_UTF16LE: [u8; 2] = [0xFF, 0xFE];

    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 2 {
            return None;
        }

        if slice.len() >= 3 && slice[0..3] == Self::BOM_UTF8 {
            return Some(Self::Utf8);
        }

        let sub_slice = &slice[0..2];
        if sub_slice == Self::BOM_UTF16BE {
            Some(Self::Utf16be)
        } else if sub_slice == Self::BOM_UTF16LE {
            Some(Self::Utf16le)
        } else {
            None
        }
    }

    pub fn bytes(self) -> &'static [u8] {
        match &self {
            Self::Utf8 => &Self::BOM_UTF8,
            Self::Utf16be => &Self::BOM_UTF16BE,
            Self::Utf16le => &Self::BOM_UTF16LE,
        }
    }

    pub fn encoder(self) -> EncodingRef {
        match &self {
            Self::Utf8 => encoding::all::UTF_8,
            Self::Utf16be => encoding::all::UTF_16BE,
            Self::Utf16le => encoding::all::UTF_16LE,
        }
    }

    pub fn len(self) -> usize {
        self.bytes().len()
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::encoding::{get_encoder, Bom, RgEncoding};

    #[test]
    fn test_bom_handles_empty_slices() {
        assert_eq!(Bom::from_slice(&[]), None);
    }

    #[test]
    fn test_bom_handles_small_slices() {
        assert_eq!(Bom::from_slice(&[0x1]), None);
        assert_eq!(Bom::from_slice(&[0x1, 0x2]), None);
        assert_eq!(Bom::from_slice(&[0x1, 0x2, 0x3]), None);
    }

    #[test]
    fn test_bom_detects_utf8_bom() {
        assert_eq!(Bom::from_slice(&[0x01, 0xEF, 0xBB, 0xBF]), None);
        assert_eq!(Bom::from_slice(&[0xEF, 0xBB, 0xBF]), Some(Bom::Utf8));
        assert_eq!(
            Bom::from_slice(&[0xEF, 0xBB, 0xBF, 0x63, 0x64, 0x65]),
            Some(Bom::Utf8)
        );
    }

    #[test]
    fn test_bom_detects_utf16be_bom() {
        assert_eq!(Bom::from_slice(&[0x01, 0xFE, 0xFF]), None);
        assert_eq!(Bom::from_slice(&[0xFE, 0xFF]), Some(Bom::Utf16be));
        assert_eq!(
            Bom::from_slice(&[0xFE, 0xFF, 0x63, 0x64, 0x65]),
            Some(Bom::Utf16be)
        );
    }

    #[test]
    fn test_bom_detects_utf16le_bom() {
        assert_eq!(Bom::from_slice(&[0x01, 0xFF, 0xFE]), None);
        assert_eq!(Bom::from_slice(&[0xFF, 0xFE]), Some(Bom::Utf16le));
        assert_eq!(
            Bom::from_slice(&[0xFF, 0xFE, 0x63, 0x64, 0x65]),
            Some(Bom::Utf16le)
        );
    }

    #[test]
    fn test_bom_returns_bom_len() {
        assert_eq!(Bom::Utf8.len(), 3);
        assert_eq!(Bom::Utf16be.len(), 2);
        assert_eq!(Bom::Utf16le.len(), 2);
    }

    #[test]
    fn test_bom_returns_encoder() {
        assert_eq!(Bom::Utf8.encoder().name(), "utf-8");
        assert_eq!(Bom::Utf16be.encoder().name(), "utf-16be");
        assert_eq!(Bom::Utf16le.encoder().name(), "utf-16le");
    }

    //
    // get_encoder
    //

    macro_rules! assert_encoder {
        ($bytes:expr, $rg_enc:expr, $expected:expr) => {
            let (bom, enc) = get_encoder($bytes, $rg_enc);
            assert_eq!((bom, enc.name()), $expected);
        };
    }

    #[test]
    fn test_get_encoder() {
        // falls back on empty
        assert_encoder!(&[], &RgEncoding::None, (None, "utf-8"));

        // BOMs (always takes preference, even if RgEncoding is passed)
        assert_encoder!(
            &Bom::BOM_UTF8,
            &RgEncoding::None,
            (Some(Bom::Utf8), "utf-8")
        );
        assert_encoder!(
            &Bom::BOM_UTF16BE,
            &RgEncoding::None,
            (Some(Bom::Utf16be), "utf-16be")
        );
        assert_encoder!(
            &Bom::BOM_UTF16LE,
            &RgEncoding::None,
            (Some(Bom::Utf16le), "utf-16le")
        );
        assert_encoder!(
            &Bom::BOM_UTF8,
            &RgEncoding::Some(encoding::all::ASCII),
            (Some(Bom::Utf8), "utf-8")
        );
        assert_encoder!(
            &Bom::BOM_UTF16BE,
            &RgEncoding::Some(encoding::all::ASCII),
            (Some(Bom::Utf16be), "utf-16be")
        );
        assert_encoder!(
            &Bom::BOM_UTF16LE,
            &RgEncoding::Some(encoding::all::ASCII),
            (Some(Bom::Utf16le), "utf-16le")
        );

        // RgEncoding (should default to this)
        assert_encoder!(
            &[0x1, 0x2, 0x3, 0x4],
            &RgEncoding::Some(encoding::all::EUC_JP),
            (None, "euc-jp")
        );
        assert_encoder!(
            &[0x1, 0x2, 0x3, 0x4],
            &RgEncoding::Some(encoding::all::ASCII),
            (None, "ascii")
        );
    }
}
