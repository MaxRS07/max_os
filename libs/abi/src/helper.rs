/// performs a bitwise conversion from isize to usize
fn as_usize_bytes(value: isize) -> usize {
    usize::from_ne_bytes(value.to_ne_bytes())
}
fn as_isize_bytes(value: usize) -> isize {
    isize::from_ne_bytes(value.to_ne_bytes())
}
