pub mod item;
pub mod sub_item;

use unicode_width::UnicodeWidthStr;

pub use item::*;
pub use sub_item::*;

#[macro_export]
macro_rules! format_line_number {
    ($content:expr) => {
        format!("{}:", $content)
    };
}

pub fn line_count(available_width: usize, text: impl AsRef<str>) -> usize {
    #[cfg(not(release))]
    assert!(available_width != 0);

    let line_width = text.as_ref().width();
    // lines that wrap
    let mut count = line_width / available_width;
    // any remainder on the last line
    if line_width % available_width > 0 {
        count += 1;
    }

    count
}
