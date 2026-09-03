#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LockState {
    Unlocked,
    Shared,
    Exclusive,
}
impl From<u8> for LockState {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Unlocked,
            1 => Self::Shared,
            _ => Self::Exclusive,
        }
    }
}
