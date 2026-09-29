use fs::vfs::{self, Vfs};

// Context for the syscall operations
/// Kernel context struct
pub struct KernelContext<'a> {
    vfs: &'a mut vfs::Vfs<'static>,
    // net: net interface
    //
}

impl<'a> KernelContext<'a> {
    pub fn new(vfs: &'a mut Vfs<'static>) -> Self {
        Self { vfs }
    }
}
