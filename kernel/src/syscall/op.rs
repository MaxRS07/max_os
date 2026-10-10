use alloc::{format, string::String};

use crate::syscall::{SyscallArgs, context::KernelContext, error::SyscallError};

pub(crate) trait SysOp: Sized + TryFrom<usize, Error = SyscallError> {
    fn call(&self, ktx: &mut KernelContext, args: SyscallArgs) -> Result<(), SyscallError>;
}

/* Arg Utils */

pub unsafe fn str_from_args(address: usize, len: usize) -> Result<String, SyscallError> {
    unsafe {
        let str_ptr = address as *const u8;
        let bytes = core::slice::from_raw_parts(str_ptr, len);
        let byte_str = str::from_utf8(bytes).map_err(|err| {
            SyscallError::InvalidArg(format!("Failed to parse argument of type string: {err}"))
        })?;
        Ok(String::from(byte_str))
    }
}

pub fn exit(ktx: &mut KernelContext, code: isize) {
    let frame = ktx.trap_frame();
    // write first return to a0
    // bytewise convert to usize, a0 is always the exit code
    // and will be interpereted as isize in userspace
    frame[5] = usize_bytes(code);
}
/// performs a bitwise conversion from isize to usize
fn usize_bytes(value: isize) -> usize {
    usize::from_ne_bytes(value.to_ne_bytes())
}

#[macro_export]
macro_rules! write_return {
    ( $ktx:expr, $( $val:expr ),+ $(,)? ) => {{
        let ktx: &mut $crate::syscall::context::KernelContext = $ktx;
        let frame = ktx.trap_frame();
        let mut reg = 6; // a1
        $(
            core::debug_assert!(reg <= 12, "write_return! takes at most 7 values (a1-a7)");
            frame[reg] = ($val) as isize as usize;
            reg += 1;
        )+
    }};
}
