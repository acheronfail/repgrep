#[derive(Debug, Copy, Clone)]
pub enum PrintableStyle {
    Common,
    Verbose,
}

impl PrintableStyle {
    pub fn swap(self) -> Self {
        match self {
            PrintableStyle::Common => PrintableStyle::Verbose,
            PrintableStyle::Verbose => PrintableStyle::Common,
        }
    }
}

pub trait Printable<T> {
    fn to_printable(&self, style: PrintableStyle) -> T;
}

impl Printable<char> for char {
    fn to_printable(&self, style: PrintableStyle) -> char {
        match style {
            PrintableStyle::Common => match self {
                // Print common whitespace as symbols
                '\x09' => '→', // HT (Horizontal Tab)
                '\x0A' => '¬',  // LF (Line feed)
                '\x0D' => '¤',  // CR (Carriage return)
                '\x20' => '␣', // SP (Space)

                // Print other non-printable whitespace with a replacement
                '\x00' | '\x01' | '\x02' | '\x03' | '\x04' | '\x05' | '\x06' | '\x07' | '\x08'
                | '\x0B' | '\x0C' | '\x0E' | '\x0F' | '\x10' | '\x11' | '\x12' | '\x13'
                | '\x14' | '\x15' | '\x16' | '\x17' | '\x18' | '\x19' | '\x1A' | '\x1B'
                | '\x1C' | '\x1D' | '\x1E' | '\x1F' | '\x7F' => '•',

                _ => *self,
            },
            PrintableStyle::Verbose => match self {
                '\x00' => '␀', // NULL (Null character)
                '\x01' => '␁', // SOH (Start of Header)
                '\x02' => '␂', // STX (Start of Text)
                '\x03' => '␃', // ETX (End of Text)
                '\x04' => '␄', // EOT (End of Trans.)
                '\x05' => '␅', // ENQ (Enquiry)
                '\x06' => '␆', // ACK (Acknowledgement)
                '\x07' => '␇', // BEL (Bell)
                '\x08' => '␈', // BS (Backspace)
                '\x09' => '␉', // HT (Horizontal Tab)
                '\x0A' => '␊', // LF (Line feed)
                '\x0B' => '␋', // VT (Vertical Tab)
                '\x0C' => '␌', // FF (Form feed)
                '\x0D' => '␍', // CR (Carriage return)
                '\x0E' => '␎', // SO (Shift Out)
                '\x0F' => '␏', // SI (Shift In)
                '\x10' => '␐', // DLE (Data link escape)
                '\x11' => '␑', // DC1 (Device control 1)
                '\x12' => '␒', // DC2 (Device control 2)
                '\x13' => '␓', // DC3 (Device control 3)
                '\x14' => '␔', // DC4 (Device control 4)
                '\x15' => '␕', // NAK (Negative acknowl.)
                '\x16' => '␖', // SYN (Synchronous idle)
                '\x17' => '␗', // ETB (End of trans. block)
                '\x18' => '␘', // CAN (Cancel)
                '\x19' => '␙', // EM (End of medium)
                '\x1A' => '␚', // SUB (Substitute)
                '\x1B' => '␛', // ESC (Escape)
                '\x1C' => '␜', // FS (File separator)
                '\x1D' => '␝', // GS (Group separator)
                '\x1E' => '␞', // RS (Record separator)
                '\x1F' => '␟', // US (Unit separator)
                '\x20' => '␠', // SP (Space)
                '\x7F' => '␡', // DEL (Delete)
                _ => *self,
            },
        }
    }
}

impl Printable<String> for &str {
    fn to_printable(&self, style: PrintableStyle) -> String {
        self.chars().map(|ch| ch.to_printable(style)).collect()
    }
}

impl Printable<String> for String {
    fn to_printable(&self, style: PrintableStyle) -> String {
        self.as_str().to_printable(style)
    }
}
