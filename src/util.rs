pub fn clamp(val: usize, min: usize, max: usize) -> usize {
    if val <= min {
        min
    } else if val >= max {
        max
    } else {
        val
    }
}

pub fn byte_pos_from_char_pos(s: &String, char_pos: usize) -> usize {
    let mut idx = 0;
    for (i, ch) in s.chars().enumerate() {
        if i == char_pos {
            break;
        }

        idx += ch.len_utf8();
    }

    idx
}
