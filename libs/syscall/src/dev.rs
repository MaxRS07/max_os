use abi::error::Errno;

use crate::syscall;

pub const DEVICE_CALL_ID: usize = 0;

pub const POWEROFF_OP: usize = 0;
pub const TEST_OP: usize = 1;
pub fn poweroff() {
    let _ = syscall!(DEVICE_CALL_ID, POWEROFF_OP);
}

pub fn test() -> Result<(), Errno> {
    syscall!(DEVICE_CALL_ID, TEST_OP)?;
    Ok(())
}
