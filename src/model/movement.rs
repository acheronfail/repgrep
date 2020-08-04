#[derive(Debug, Eq, PartialEq)]
pub enum Direction {
    Forward,
    Backward,
}

/// Defines basic movement types in the main matches list.
#[derive(Debug, Eq, PartialEq)]
pub enum Movement {
    /// Move to the previous match.
    Prev,
    /// Move to the next match.
    Next,
    /// Move to the previous line.
    PrevLine,
    /// Move to the next line.
    NextLine,
    /// Move to the previous file.
    PrevFile,
    /// Move to the next file.
    NextFile,
    /// Move forward `n` items.
    Forward(u16),
    /// Move backward `n` items.
    Backward(u16),
}

impl Movement {
    pub fn is_forward(&self) -> bool {
        matches!(self.direction(), Direction::Forward)
    }

    pub fn direction(&self) -> Direction {
        match self {
            Movement::Prev | Movement::PrevLine | Movement::PrevFile | Movement::Backward(_) => {
                Direction::Backward
            }
            Movement::Next | Movement::NextLine | Movement::NextFile | Movement::Forward(_) => {
                Direction::Forward
            }
        }
    }
}
