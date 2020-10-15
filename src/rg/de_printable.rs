use crate::model::{Printable, PrintableStyle};
use crate::rg::de::ArbitraryData;

impl Printable for ArbitraryData {
    fn to_printable(&self, style: PrintableStyle) -> String {
        self.lossy_utf8().to_printable(style)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{Printable, PrintableStyle};
    use crate::rg::de::ArbitraryData;

    const NON_PRINTABLE_WHITESPACE: &str = "\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F\x20\x7F";

    #[test]
    fn test_printable_text() {
        let data = ArbitraryData::new_with_text(NON_PRINTABLE_WHITESPACE.to_string());
        assert_eq!(
            data.to_printable(PrintableStyle::Verbose),
            "␀␁␂␃␄␅␆␇␈␉␊␋␌␍␎␏␐␑␒␓␔␕␖␗␘␙␚␛␜␝␞␟␠␡"
        );
        assert_eq!(
            data.to_printable(PrintableStyle::Common),
            "•••••••••→¬••¤••••••••••••••••••␣•"
        );
    }

    #[test]
    fn test_printable_base64() {
        let data = ArbitraryData::new_with_base64(base64::encode(NON_PRINTABLE_WHITESPACE));
        assert_eq!(
            data.to_printable(PrintableStyle::Verbose),
            "␀␁␂␃␄␅␆␇␈␉␊␋␌␍␎␏␐␑␒␓␔␕␖␗␘␙␚␛␜␝␞␟␠␡"
        );
        assert_eq!(
            data.to_printable(PrintableStyle::Common),
            "•••••••••→¬••¤••••••••••••••••••␣•"
        );
    }
}
