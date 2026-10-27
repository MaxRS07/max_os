/// The user stack allocation
#[derive(Debug)]
pub struct UserStack {
    /// The virtual address of the bottom of the stack
    base: usize,
    /// The virtual address of the top of the stack
    top: usize,
}
