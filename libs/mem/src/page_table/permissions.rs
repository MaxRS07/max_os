/// Page permission bitflags
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PagePermissions(u8);

impl PagePermissions {
    const READ: u8 = 1;
    const WRITE: u8 = 2;
    const EXECUTE: u8 = 4;
    const USER: u8 = 8;
}
