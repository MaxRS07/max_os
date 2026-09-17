use crate::mm::heap::page_table::table::Table;

/// Address space owns a root table. Address spaces are assigned per process
pub struct AddressSpace {
    asid: u16,
    root_frame: usize,
}

impl AddressSpace {
    pub fn new(root_frame: usize, asid: u16) -> Self {
        Self { asid, root_frame }
    }
}
