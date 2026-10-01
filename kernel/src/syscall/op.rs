use crate::syscall::{SyscallArgs, context::KernelContext};

pub(crate) trait SysOp {
    fn call(&self, ktx: KernelContext, args: SyscallArgs);
}

/* Arg Utils */

pub unsafe fn str_from_args(address: usize, len: usize) -> &str {
    unsafe {
        let str_ptr = address as *const u8;
        let bytes = core::slice::from_raw_parts(str_ptr, len);
        let byte_str = str::from_utf8(bytes).unwrap(); // TODO: Actually handle this 
        return byte_str;
    }
}
