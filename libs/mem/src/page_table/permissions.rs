/// Page permission bitflags
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PagePermissions(u8);

const READ: u8 = 1;
const WRITE: u8 = 2;
const EXECUTE: u8 = 4;
const USER: u8 = 8;

impl PagePermissions {
    pub const READ: PagePermissions = PagePermissions(READ);
    pub const WRITE: PagePermissions = PagePermissions(WRITE);
    pub const EXECUTE: PagePermissions = PagePermissions(EXECUTE);
    pub const USER: PagePermissions = PagePermissions(USER);
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
