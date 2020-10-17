type OneLine = bool;

#[derive(Debug, Copy, Clone)]
pub enum PrintableStyle {
    None,
    Common(OneLine),
    Verbose(OneLine),
}

impl PrintableStyle {
    /// Cycles through each possible value of a `PrintableStyle`.
    pub fn cycle(self) -> Self {
        match self {
            PrintableStyle::None => PrintableStyle::Common(false),
            PrintableStyle::Common(false) => PrintableStyle::Verbose(false),
            PrintableStyle::Verbose(false) => PrintableStyle::Common(true),
            PrintableStyle::Common(true) => PrintableStyle::Verbose(true),
            PrintableStyle::Verbose(true) => PrintableStyle::None,
        }
    }

    /// Returns the "one line" representation of the current `PrintableStyle`.
    pub fn one_line(self) -> Self {
        match self {
            PrintableStyle::None => PrintableStyle::Common(true),
            PrintableStyle::Common(_) => PrintableStyle::Common(true),
            PrintableStyle::Verbose(_) => PrintableStyle::Verbose(true),
        }
    }
}

pub trait Printable {
    fn to_printable(&self, style: PrintableStyle) -> String;
}

impl Printable for char {
    fn to_printable(&self, style: PrintableStyle) -> String {
        match style {
            PrintableStyle::None => String::from(*self),
            PrintableStyle::Common(oneline) => match self {
                // Print common whitespace as symbols
                '\x09' => String::from("→"), // HT (Horizontal Tab)
                '\x0A' => String::from(if oneline { "¬" } else { "¬\n" }), // LF (Line feed)
                '\x0D' => String::from("¤"),  // CR (Carriage return)
                '\x20' => String::from("␣"), // SP (Space)

                // Print other non-printable whitespace with a replacement
                '\x00' | '\x01' | '\x02' | '\x03' | '\x04' | '\x05' | '\x06' | '\x07' | '\x08'
                | '\x0B' | '\x0C' | '\x0E' | '\x0F' | '\x10' | '\x11' | '\x12' | '\x13'
                | '\x14' | '\x15' | '\x16' | '\x17' | '\x18' | '\x19' | '\x1A' | '\x1B'
                | '\x1C' | '\x1D' | '\x1E' | '\x1F' | '\x7F' => String::from("•"),

                _ => String::from(*self),
            },
            PrintableStyle::Verbose(oneline) => match self {
                '\x00' => String::from("␀"), // NULL (Null character)
                '\x01' => String::from("␁"), // SOH (Start of Header)
                '\x02' => String::from("␂"), // STX (Start of Text)
                '\x03' => String::from("␃"), // ETX (End of Text)
                '\x04' => String::from("␄"), // EOT (End of Trans.)
                '\x05' => String::from("␅"), // ENQ (Enquiry)
                '\x06' => String::from("␆"), // ACK (Acknowledgement)
                '\x07' => String::from("␇"), // BEL (Bell)
                '\x08' => String::from("␈"), // BS (Backspace)
                '\x09' => String::from("␉"), // HT (Horizontal Tab)
                '\x0A' => String::from(if oneline { "␊" } else { "␊\n" }), // LF (Line feed)
                '\x0B' => String::from("␋"), // VT (Vertical Tab)
                '\x0C' => String::from("␌"), // FF (Form feed)
                '\x0D' => String::from("␍"), // CR (Carriage return)
                '\x0E' => String::from("␎"), // SO (Shift Out)
                '\x0F' => String::from("␏"), // SI (Shift In)
                '\x10' => String::from("␐"), // DLE (Data link escape)
                '\x11' => String::from("␑"), // DC1 (Device control 1)
                '\x12' => String::from("␒"), // DC2 (Device control 2)
                '\x13' => String::from("␓"), // DC3 (Device control 3)
                '\x14' => String::from("␔"), // DC4 (Device control 4)
                '\x15' => String::from("␕"), // NAK (Negative acknowl.)
                '\x16' => String::from("␖"), // SYN (Synchronous idle)
                '\x17' => String::from("␗"), // ETB (End of trans. block)
                '\x18' => String::from("␘"), // CAN (Cancel)
                '\x19' => String::from("␙"), // EM (End of medium)
                '\x1A' => String::from("␚"), // SUB (Substitute)
                '\x1B' => String::from("␛"), // ESC (Escape)
                '\x1C' => String::from("␜"), // FS (File separator)
                '\x1D' => String::from("␝"), // GS (Group separator)
                '\x1E' => String::from("␞"), // RS (Record separator)
                '\x1F' => String::from("␟"), // US (Unit separator)
                '\x20' => String::from("␠"), // SP (Space)
                '\x7F' => String::from("␡"), // DEL (Delete)
                _ => String::from(*self),
            },
        }
    }
}

impl Printable for &str {
    fn to_printable(&self, style: PrintableStyle) -> String {
        self.chars().map(|ch| ch.to_printable(style)).collect()
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
