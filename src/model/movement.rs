/// Defines basic movement types in the main matches list.
#[derive(Debug, Eq, PartialEq)]
pub enum Movement {
  /// Move to the previous item.
  Prev,
  /// Move to the next item.
  Next,
  /// Move to the previous file.
  PrevFile,
  /// Move to the next file.
  NextFile,
  /// Move forward `n` items.
  Forward(u16),
  /// Move backward `n` items.
  Backward(u16),
}
