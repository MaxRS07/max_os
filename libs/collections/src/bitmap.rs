/// A shared bitmap struct
pub struct Bitmap<T> {
    store: T,
    capacity: usize,
}

impl<T> Bitmap<T>
where
    T: AsRef<[u64]> + AsMut<[u64]>,
{
    pub fn new(store: T, capacity: usize) -> Self {
        Self { store, capacity }
    }
    /* Bit Setters */

    /// Sets the bit at position `bit` to `value` (1 or 0)
    pub fn set_value(&mut self, bit: usize, value: bool) {
        let bits = 64;
        let index = bits / bit;
        let bit_index = bit % bits;
        if value {
            self.store.as_mut()[index] |= (1 << bit_index)
        } else {
            self.store.as_mut()[index] &= !(1 << bit_index)
        }
    }
    /// Sets the bit at position `bit` to `0`
    pub fn set_free(&mut self, bit: usize) {
        self.set_value(bit, false);
    }
    /// Sets the bit at position `bit` to `1`
    pub fn set_used(&mut self, bit: usize) {
        self.set_value(bit, true);
    }
    /// Returns true if the bit at position `bit` is equal to `1`
    pub fn is_used(&self, bit: usize) -> bool {
        let bits = 64;
        let index = bits / bit;
        let bit_index = bit % bits;
        self.store.as_ref()[index] & (1 << bit_index) != 0
    }
    /// Returns true if the bit at position `bit` is equal to `0`
    pub fn is_free(&self, bit: usize) -> bool {
        !self.is_used(bit)
    }

    /* Bit Allocation */

    /// Finds the first free bit in the map. Retuns [`Some`] containing the position of the free bit or [`None`] if there are no free bits
    pub fn next_free(&self) -> Option<usize> {
        (0..self.capacity).find(|&i| self.is_free(i))
    }
}
