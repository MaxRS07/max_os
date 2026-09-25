// alignment helpers so i can stop spamming bitwise ops

/// Returns `value` aligned down to `align`
///
/// Panics if align is not a power of 2
#[inline]
#[must_use]
pub fn align_down(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    value & !(align - 1)
}

/// Returns `value` aligned up to `align`
///
/// Panics if align is not a power of 2
#[inline]
#[must_use]
pub fn aligned_up(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (value + (align - 1)) & !(align - 1)
}
