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

pub fn exit(code: i32) {
    unsafe { core::arch::asm!("mv a0, {}", in(reg) code) }
}
