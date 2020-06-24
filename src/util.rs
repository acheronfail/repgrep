pub fn clamp(val: usize, min: usize, max: usize) -> usize {
    if val <= min {
        min
    } else if val >= max {
        max
    } else {
        val
    }
}
