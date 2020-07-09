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
        .map(|b| b.encoder())
        // otherwise if the user passed an encoding use that
        .or_else(|| rg_encoding.encoder())
        // nothing so far, try detecting the encoding
        .or_else(|| {
            let (encoding, confidence, _) = chardet::detect(&bytes);
            // TODO: be able to adjust chardet confidence here
            if confidence > 0.80 {
                encoding_from_whatwg_label(charset2encoding(&encoding))
            } else {
                None
            }
        })
        // if all else fails, assume ASCII
        .unwrap_or_else(|| encoding::all::ASCII);

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

    use crate::encoding::Bom;

    #[test]
    fn it_handles_empty_slices() {
        assert_eq!(Bom::from_slice(&[]), None);
    }

    #[test]
    fn it_handles_small_slices() {
        assert_eq!(Bom::from_slice(&[0x1]), None);
        assert_eq!(Bom::from_slice(&[0x1, 0x2]), None);
        assert_eq!(Bom::from_slice(&[0x1, 0x2, 0x3]), None);
    }

    #[test]
    fn it_detects_utf8_bom() {
        assert_eq!(Bom::from_slice(&[0x01, 0xEF, 0xBB, 0xBF]), None);
        assert_eq!(Bom::from_slice(&[0xEF, 0xBB, 0xBF]), Some(Bom::Utf8));
        assert_eq!(
            Bom::from_slice(&[0xEF, 0xBB, 0xBF, 0x63, 0x64, 0x65]),
            Some(Bom::Utf8)
        );
    }

    #[test]
    fn it_detects_utf16be_bom() {
        assert_eq!(Bom::from_slice(&[0x01, 0xFE, 0xFF]), None);
        assert_eq!(Bom::from_slice(&[0xFE, 0xFF]), Some(Bom::Utf16be));
        assert_eq!(
            Bom::from_slice(&[0xFE, 0xFF, 0x63, 0x64, 0x65]),
            Some(Bom::Utf16be)
        );
    }

    #[test]
    fn it_detects_utf16le_bom() {
        assert_eq!(Bom::from_slice(&[0x01, 0xFF, 0xFE]), None);
        assert_eq!(Bom::from_slice(&[0xFF, 0xFE]), Some(Bom::Utf16le));
        assert_eq!(
            Bom::from_slice(&[0xFF, 0xFE, 0x63, 0x64, 0x65]),
            Some(Bom::Utf16le)
        );
    }

    #[test]
    fn it_returns_bom_len() {
        assert_eq!(Bom::Utf8.len(), 3);
        assert_eq!(Bom::Utf16be.len(), 2);
        assert_eq!(Bom::Utf16le.len(), 2);
    }

    #[test]
    fn it_returns_encoder() {
        assert_eq!(Bom::Utf8.encoder().name(), "utf-8");
        assert_eq!(Bom::Utf16be.encoder().name(), "utf-16be");
        assert_eq!(Bom::Utf16le.encoder().name(), "utf-16le");
    }
}
