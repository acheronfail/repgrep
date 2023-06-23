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

impl Default for PrintableStyle {
    fn default() -> Self {
        PrintableStyle::Hidden
    }
}

impl Display for PrintableStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
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

    pub fn symbol(self) -> char {
        match self {
            PrintableStyle::Hidden => 'H',
            PrintableStyle::Common(false) => 'C',
            PrintableStyle::Common(true) => 'c',
            PrintableStyle::All(false) => 'A',
            PrintableStyle::All(true) => 'a',
        }
    }
}

pub trait Printable {
    fn to_printable(&self, style: PrintableStyle) -> String;
}

impl Printable for &str {
    fn to_printable(&self, style: PrintableStyle) -> String {
        match style {
            PrintableStyle::Hidden => {
                let mut s = String::with_capacity(self.len());
                for ch in self.chars() {
                    match ch {
                        '\x00' | '\x01' | '\x02' | '\x03' | '\x04' | '\x05' | '\x06' | '\x07'
                        | '\x08' | '\x0B' | '\x0C' | '\x0E' | '\x0F' | '\x10' | '\x11' | '\x12'
                        | '\x13' | '\x14' | '\x15' | '\x16' | '\x17' | '\x18' | '\x19' | '\x1A'
                        | '\x1B' | '\x1C' | '\x1D' | '\x1E' | '\x1F' | '\x7F' => {}
                        '\x09' | '\x0D' => s.push(' '),
                        _ => s.push(ch),
                    }
                }

                s
            }

            PrintableStyle::Common(oneline) => {
                let mut s = String::with_capacity(self.len());
                for ch in self.chars() {
                    match ch {
                        // Print common whitespace as symbols
                        '\x09' => s.push('→'), // HT (Horizontal Tab)
                        '\x0A' => s.push_str(if oneline { "¬" } else { "¬\n" }), // LF (Line feed)
                        '\x0D' => s.push('¤'), // CR (Carriage return)
                        '\x20' => s.push('␣'), // SP (Space)
                        // Print other control characters with a replacement
                        '\x00' | '\x01' | '\x02' | '\x03' | '\x04' | '\x05' | '\x06' | '\x07'
                        | '\x08' | '\x0B' | '\x0C' | '\x0E' | '\x0F' | '\x10' | '\x11' | '\x12'
                        | '\x13' | '\x14' | '\x15' | '\x16' | '\x17' | '\x18' | '\x19' | '\x1A'
                        | '\x1B' | '\x1C' | '\x1D' | '\x1E' | '\x1F' | '\x7F' => s.push('•'),
                        c => s.push(c),
                    }
                }

                s
            }
            PrintableStyle::All(oneline) => {
                let mut s = String::with_capacity(self.len());
                for ch in self.chars() {
                    match ch {
                        '\x00' => s.push('␀'), // NULL (Null character)
                        '\x01' => s.push('␁'), // SOH (Start of Header)
                        '\x02' => s.push('␂'), // STX (Start of Text)
                        '\x03' => s.push('␃'), // ETX (End of Text)
                        '\x04' => s.push('␄'), // EOT (End of Trans.)
                        '\x05' => s.push('␅'), // ENQ (Enquiry)
                        '\x06' => s.push('␆'), // ACK (Acknowledgement)
                        '\x07' => s.push('␇'), // BEL (Bell)
                        '\x08' => s.push('␈'), // BS (Backspace)
                        '\x09' => s.push('␉'), // HT (Horizontal Tab)
                        '\x0A' => s.push_str(if oneline { "␊" } else { "␊\n" }), // LF (Line feed)
                        '\x0B' => s.push('␋'), // VT (Vertical Tab)
                        '\x0C' => s.push('␌'), // FF (Form feed)
                        '\x0D' => s.push('␍'), // CR (Carriage return)
                        '\x0E' => s.push('␎'), // SO (Shift Out)
                        '\x0F' => s.push('␏'), // SI (Shift In)
                        '\x10' => s.push('␐'), // DLE (Data link escape)
                        '\x11' => s.push('␑'), // DC1 (Device control 1)
                        '\x12' => s.push('␒'), // DC2 (Device control 2)
                        '\x13' => s.push('␓'), // DC3 (Device control 3)
                        '\x14' => s.push('␔'), // DC4 (Device control 4)
                        '\x15' => s.push('␕'), // NAK (Negative acknowl.)
                        '\x16' => s.push('␖'), // SYN (Synchronous idle)
                        '\x17' => s.push('␗'), // ETB (End of trans. block)
                        '\x18' => s.push('␘'), // CAN (Cancel)
                        '\x19' => s.push('␙'), // EM (End of medium)
                        '\x1A' => s.push('␚'), // SUB (Substitute)
                        '\x1B' => s.push('␛'), // ESC (Escape)
                        '\x1C' => s.push('␜'), // FS (File separator)
                        '\x1D' => s.push('␝'), // GS (Group separator)
                        '\x1E' => s.push('␞'), // RS (Record separator)
                        '\x1F' => s.push('␟'), // US (Unit separator)
                        '\x20' => s.push('␠'), // SP (Space)
                        '\x7F' => s.push('␡'), // DEL (Delete)
                        c => s.push(c),
                    }
                }

                s
            }
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

impl Printable for Vec<u8> {
    fn to_printable(&self, style: PrintableStyle) -> String {
        String::from_utf8_lossy(self).to_printable(style)
    }
}

#[cfg(test)]
mod tests {
    use base64_simd::STANDARD as base64;

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
        let data =
            ArbitraryData::new_with_base64(base64.encode_to_string(NON_PRINTABLE_WHITESPACE));
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
