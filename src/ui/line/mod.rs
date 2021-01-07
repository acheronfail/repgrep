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

#[inline]
pub fn line_count(available_width: usize, text: impl AsRef<str>) -> usize {
    #[cfg(not(release))]
    assert!(available_width != 0);

    (text.as_ref().width() / available_width) + 1
}
