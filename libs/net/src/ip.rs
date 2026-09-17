#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpAddress {
    V4([u8; 4]),
    V6([u8; 6]),
}
