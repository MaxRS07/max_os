use core::ops::{BitAnd, BitOr};

/// Page permission bitflags
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PagePermissions(u8);

impl PagePermissions {
    pub const READ: PagePermissions = PagePermissions(1);
    pub const WRITE: PagePermissions = PagePermissions(2);
    pub const EXECUTE: PagePermissions = PagePermissions(4);
    pub const USER: PagePermissions = PagePermissions(8);

    pub const RW: PagePermissions = PagePermissions(3);
    pub const RWX: PagePermissions = PagePermissions(7);
    pub const RWXU: PagePermissions = PagePermissions(15);

    pub fn raw_flags(&self) -> u8 {
        self.0
    }
    pub fn readonly(&self) -> bool {
        *self & Self::WRITE == 0
    }
    pub fn write(&self) -> bool {
        !self.readonly()
    }
    pub fn execute(&self) -> bool {
        *self & Self::EXECUTE != 0
    }
    pub fn user(&self) -> bool {
        *self & Self::USER != 0
    }
}
impl BitAnd for PagePermissions {
    type Output = u8;
    fn bitand(self, rhs: Self) -> Self::Output {
        self.0 & rhs.0
    }
}
impl BitOr for PagePermissions {
    type Output = u8;
    fn bitor(self, rhs: Self) -> Self::Output {
        self.0 & rhs.0
    }
}
impl From<u8> for PagePermissions {
    fn from(value: u8) -> Self {
        match value {
            1 => PagePermissions::READ,
            2 => PagePermissions::WRITE,
            3 => PagePermissions::EXECUTE,
            _ => PagePermissions::USER,
        }
    }
}
