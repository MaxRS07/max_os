use fs::vfs::{self, Vfs};

// Context for the syscall operations
/// Kernel context struct
pub struct KernelContext<'a> {
    vfs: &'a mut vfs::Vfs<'a>,
    // net: net interface
    //
}

impl<'a> KernelContext<'a> {
    pub fn new(vfs: &'static mut Vfs) -> Self {
        Self { vfs }
    }
}
