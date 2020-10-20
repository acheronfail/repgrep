use std::borrow::Cow;
use std::fmt::{self, Display};

use crate::rg::de::ArbitraryData;

type OneLine = bool;

#[derive(Debug, Copy, Clone)]
pub enum PrintableStyle {
    Hidden,
    Common(OneLine),
    All(OneLine),
}

impl Display for PrintableStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrintableStyle::Hidden => write!(f, "H"),
            PrintableStyle::Common(false) => write!(f, "C"),
            PrintableStyle::Common(true) => write!(f, "c"),
            PrintableStyle::All(false) => write!(f, "A"),
            PrintableStyle::All(true) => write!(f, "a"),
        }
    }
}

impl PrintableStyle {
    /// Cycles through each possible value of a `PrintableStyle`.
    pub fn cycle(self) -> Self {
        match self {
            PrintableStyle::Hidden => PrintableStyle::Common(false),
            PrintableStyle::Common(false) => PrintableStyle::Common(true),
            PrintableStyle::Common(true) => PrintableStyle::All(false),
            PrintableStyle::All(false) => PrintableStyle::All(true),
            PrintableStyle::All(true) => PrintableStyle::Hidden,
        }
    }

    /// Returns the "one line" representation of the current `PrintableStyle`.
    pub fn as_one_line(self) -> Self {
        match self {
            PrintableStyle::Hidden => PrintableStyle::Common(true),
            PrintableStyle::Common(_) => PrintableStyle::Common(true),
            PrintableStyle::All(_) => PrintableStyle::All(true),
        }
    }

    /// Returns the "one line" representation of the current `PrintableStyle`.
    pub fn is_one_line(self) -> bool {
        matches!(
            self,
            PrintableStyle::Common(true) | PrintableStyle::All(true)
        )
    }
}

pub trait Printable {
    fn to_printable(&self, style: PrintableStyle) -> String;
}

impl Printable for &str {
    fn to_printable(&self, style: PrintableStyle) -> String {
        match style {
            PrintableStyle::Hidden => self
                // Print common control characters as a single space
                .replace(&['\x09', '\x0D'][..], " ")
                // Strip all other control characters to hide them
                .replace(
                    &[
                        '\x00', '\x01', '\x02', '\x03', '\x04', '\x05', '\x06', '\x07', '\x08',
                        '\x0B', '\x0C', '\x0E', '\x0F', '\x10', '\x11', '\x12', '\x13', '\x14',
                        '\x15', '\x16', '\x17', '\x18', '\x19', '\x1A', '\x1B', '\x1C', '\x1D',
                        '\x1E', '\x1F', '\x7F',
                    ][..],
                    "",
                ),

            PrintableStyle::Common(oneline) => self
                // Print common whitespace as symbols
                .replace('\x09', "→") // HT (Horizontal Tab)
                .replace('\x0A', if oneline { "¬" } else { "¬\n" }) // LF (Line feed)
                .replace('\x0D', "¤") // CR (Carriage return)
                .replace('\x20', "␣") // SP (Space)
                // Print other control characters with a replacement
                .replace(
                    &[
                        '\x00', '\x01', '\x02', '\x03', '\x04', '\x05', '\x06', '\x07', '\x08',
                        '\x0B', '\x0C', '\x0E', '\x0F', '\x10', '\x11', '\x12', '\x13', '\x14',
                        '\x15', '\x16', '\x17', '\x18', '\x19', '\x1A', '\x1B', '\x1C', '\x1D',
                        '\x1E', '\x1F', '\x7F',
                    ][..],
                    "•",
                ),
            PrintableStyle::All(oneline) => self
                .replace('\x00', "␀") // NULL (Null character)
                .replace('\x01', "␁") // SOH (Start of Header)
                .replace('\x02', "␂") // STX (Start of Text)
                .replace('\x03', "␃") // ETX (End of Text)
                .replace('\x04', "␄") // EOT (End of Trans.)
                .replace('\x05', "␅") // ENQ (Enquiry)
                .replace('\x06', "␆") // ACK (Acknowledgement)
                .replace('\x07', "␇") // BEL (Bell)
                .replace('\x08', "␈") // BS (Backspace)
                .replace('\x09', "␉") // HT (Horizontal Tab)
                .replace('\x0A', if oneline { "␊" } else { "␊\n" }) // LF (Line feed)
                .replace('\x0B', "␋") // VT (Vertical Tab)
                .replace('\x0C', "␌") // FF (Form feed)
                .replace('\x0D', "␍") // CR (Carriage return)
                .replace('\x0E', "␎") // SO (Shift Out)
                .replace('\x0F', "␏") // SI (Shift In)
                .replace('\x10', "␐") // DLE (Data link escape)
                .replace('\x11', "␑") // DC1 (Device control 1)
                .replace('\x12', "␒") // DC2 (Device control 2)
                .replace('\x13', "␓") // DC3 (Device control 3)
                .replace('\x14', "␔") // DC4 (Device control 4)
                .replace('\x15', "␕") // NAK (Negative acknowl.)
                .replace('\x16', "␖") // SYN (Synchronous idle)
                .replace('\x17', "␗") // ETB (End of trans. block)
                .replace('\x18', "␘") // CAN (Cancel)
                .replace('\x19', "␙") // EM (End of medium)
                .replace('\x1A', "␚") // SUB (Substitute)
                .replace('\x1B', "␛") // ESC (Escape)
                .replace('\x1C', "␜") // FS (File separator)
                .replace('\x1D', "␝") // GS (Group separator)
                .replace('\x1E', "␞") // RS (Record separator)
                .replace('\x1F', "␟") // US (Unit separator)
                .replace('\x20', "␠") // SP (Space)
                .replace('\x7F', "␡"), // DEL (Delete)
        }
    }
}

impl Printable for &String {
    fn to_printable(&self, style: PrintableStyle) -> String {
        self.as_str().to_printable(style)
    }
}

impl Printable for String {
    fn to_printable(&self, style: PrintableStyle) -> String {
        self.as_str().to_printable(style)
    }
}

impl<'a> Printable for Cow<'a, str> {
    fn to_printable(&self, style: PrintableStyle) -> String {
        self.to_string().to_printable(style)
    }
}

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
    fn test_printable() {
        assert_eq!(
            NON_PRINTABLE_WHITESPACE.to_printable(PrintableStyle::Hidden),
            " \n  "
        );
        assert_eq!(
            NON_PRINTABLE_WHITESPACE.to_printable(PrintableStyle::All(true)),
            "␀␁␂␃␄␅␆␇␈␉␊␋␌␍␎␏␐␑␒␓␔␕␖␗␘␙␚␛␜␝␞␟␠␡"
        );
        assert_eq!(
            NON_PRINTABLE_WHITESPACE.to_printable(PrintableStyle::All(false)),
            "␀␁␂␃␄␅␆␇␈␉␊\n␋␌␍␎␏␐␑␒␓␔␕␖␗␘␙␚␛␜␝␞␟␠␡"
        );
        assert_eq!(
            NON_PRINTABLE_WHITESPACE.to_printable(PrintableStyle::Common(true)),
            "•••••••••→¬••¤••••••••••••••••••␣•"
        );
        assert_eq!(
            NON_PRINTABLE_WHITESPACE.to_printable(PrintableStyle::Common(false)),
            "•••••••••→¬\n••¤••••••••••••••••••␣•"
        );
    }

    #[test]
    fn test_printable_oneline() {
        assert_eq!("\n".to_printable(PrintableStyle::Hidden), "\n");
        assert_eq!("\n".to_printable(PrintableStyle::Common(false)), "¬\n");
        assert_eq!("\n".to_printable(PrintableStyle::Common(true)), "¬");
        assert_eq!("\n".to_printable(PrintableStyle::All(false)), "␊\n");
        assert_eq!("\n".to_printable(PrintableStyle::All(true)), "␊");
    }

    #[test]
    fn test_printable_text() {
        let data = ArbitraryData::new_with_text(NON_PRINTABLE_WHITESPACE.to_string());
        assert_eq!(data.to_printable(PrintableStyle::Hidden), " \n  ");
        assert_eq!(
            data.to_printable(PrintableStyle::All(true)),
            "␀␁␂␃␄␅␆␇␈␉␊␋␌␍␎␏␐␑␒␓␔␕␖␗␘␙␚␛␜␝␞␟␠␡"
        );
        assert_eq!(
            data.to_printable(PrintableStyle::Common(true)),
            "•••••••••→¬••¤••••••••••••••••••␣•"
        );
    }

    #[test]
    fn test_printable_base64() {
        let data = ArbitraryData::new_with_base64(base64::encode(NON_PRINTABLE_WHITESPACE));
        assert_eq!(data.to_printable(PrintableStyle::Hidden), " \n  ");
        assert_eq!(
            data.to_printable(PrintableStyle::All(true)),
            "␀␁␂␃␄␅␆␇␈␉␊␋␌␍␎␏␐␑␒␓␔␕␖␗␘␙␚␛␜␝␞␟␠␡"
        );
        assert_eq!(
            data.to_printable(PrintableStyle::Common(true)),
            "•••••••••→¬••¤••••••••••••••••••␣•"
        );
    }
}
