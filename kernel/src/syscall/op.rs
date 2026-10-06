use alloc::string::String;

use crate::syscall::{SyscallArgs, context::KernelContext, error::SyscallError};

pub(crate) trait SysOp: Sized + TryFrom<usize, Error = SyscallError> {
    fn call(&self, ktx: &mut KernelContext, args: SyscallArgs);
}

/* Arg Utils */

pub unsafe fn str_from_args(address: usize, len: usize) -> String {
    unsafe {
        let str_ptr = address as *const u8;
        let bytes = core::slice::from_raw_parts(str_ptr, len);
        let byte_str = str::from_utf8(bytes).unwrap(); // TODO: Actually handle this 
        String::from(byte_str)
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

/// This macro writes 8 return values through the a0-a7 registers
#[macro_export]
macro_rules! exit {
    ( $arg0_ktx:expr, $( $rest:expr ),+ $(,)? ) => {
        let ktx: KernelContext = $arg0_ktx;
        let args = [ $( $rest as usize ),+ ];
        let len = args.len();
        core::debug_assert!(len <= 7, "exit! takes at most 7 additional arguments (8 total including ktx)");
        // Writes args starting at a0
        ktx.trap_frame()[5..=len].copy_from_slice(&args);
    };
}
