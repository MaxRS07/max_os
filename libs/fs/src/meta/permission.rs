#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// Wrapper struct for Unix style permission bitflags
pub struct Permissions(u16);

impl Permissions {
    /// Used to mark files that should inherit permissions from their parent directory
    pub const INHERIT: u16 = 1 << 15;

    pub const OWNER_MASK: u16 = 0o700;
    pub const GROUP_MASK: u16 = 0o070;
    pub const PUBLIC_MASK: u16 = 0o007;

    pub const OWNER: u16 = 6;
    pub const GROUP: u16 = 3;
    pub const PUBLIC: u16 = 0;

    pub const EXECUTE: u16 = 1;
    pub const READ: u16 = 2;
    pub const WRITE: u16 = 4;

    pub fn without_umask(self, umask: Self) -> Self {
        Self(self.0 & !umask.0)
    }
    pub fn set_raw(&mut self, value: u16) {
        self.0 &= !value;
        self.0 |= value & (Self::OWNER_MASK | Self::GROUP_MASK | Self::PUBLIC_MASK)
    }
    pub fn set_owner(&mut self, r: bool, w: bool, x: bool) {
        let rwx = Self::to_raw_flags(r, w, x);
        self.0 &= !Self::OWNER_MASK;
        self.0 |= rwx << Self::OWNER;
    }
    pub fn set_group(&mut self, r: bool, w: bool, x: bool) {
        let rwx = Self::to_raw_flags(r, w, x);
        self.0 &= !Self::GROUP_MASK;
        self.0 |= rwx << Self::GROUP;
    }
    pub fn set_public(&mut self, r: bool, w: bool, x: bool) {
        let rwx = Self::to_raw_flags(r, w, x);
        self.0 &= !Self::PUBLIC_MASK;
        self.0 |= rwx << Self::PUBLIC;
    }
    fn with_owner(&self, r: bool, w: bool, x: bool) -> Self {
        let mut new = *self;
        new.set_owner(r, w, x);
        new
    }
    fn with_group(&self, r: bool, w: bool, x: bool) -> Self {
        let mut new = *self;
        new.set_group(r, w, x);
        new
    }
    fn with_public(&self, r: bool, w: bool, x: bool) -> Self {
        let mut new = *self;
        new.set_public(r, w, x);
        new
    }
    fn from_owner(r: bool, w: bool, x: bool) -> Self {
        let mut new = Self(0);
        new.set_owner(r, w, x);
        new
    }
    fn from_group(r: bool, w: bool, x: bool) -> Self {
        let mut new = Self(0);
        new.set_group(r, w, x);
        new
    }
    fn from_public(r: bool, w: bool, x: bool) -> Self {
        let mut new = Self(0);
        new.set_public(r, w, x);
        new
    }
    fn to_raw_flags(r: bool, w: bool, x: bool) -> u16 {
        (Self::READ * r as u16) + (Self::WRITE * w as u16) + (Self::EXECUTE * x as u16)
    }
    fn inherit(&self) -> bool {
        self.0 & Self::INHERIT != 0
    }
}
