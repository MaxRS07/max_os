use abi::{error::Errno, helper::as_usize_bytes};
use alloc::{format, string::String, vec};
use mem::page_table::table::{USER_VA_END, USER_VA_START};

use crate::syscall::{self, SyscallArgs, context::KernelContext, error::SyscallError};

pub(crate) trait SysOp: Sized + TryFrom<usize, Error = Errno> {
    fn call(&self, ktx: &mut KernelContext, args: SyscallArgs) -> Result<(), SyscallError>;
}

const ARG_MAX_LENGTH: usize = 0xFF;

/* Arg Helpers */
// TODO: Move to the syscall library

/// TODO: check for user page permissions. IMPORTANT
pub unsafe fn str_from_args(address: usize, len: usize) -> Result<String, SyscallError> {
    if len > ARG_MAX_LENGTH {
        return Err(SyscallError::InvalidArg(format!(
            "Arg string exceeds max length: {len} > 255"
        )));
    }
    let mut buffer = vec![0u8; len];
    copy_from_user(&mut buffer, address)?;
    let byte_str = str::from_utf8(&buffer).map_err(|err| {
        SyscallError::InvalidArg(format!("Failed to parse argument of type string: {err}"))
    })?;
    Ok(String::from(byte_str))
}

pub fn exit(ktx: &mut KernelContext, code: isize) {
    let frame = ktx.trap_frame();
    // write first return to a0
    // bytewise convert to usize, a0 is always the exit code
    // and will be interpereted as isize in userspace
    frame.a[0] = as_usize_bytes(code);
}

#[macro_export]
macro_rules! write_return {
    ($ktx:expr, $($val:expr),+ $(,)?) => {
        $ktx.trap_frame().set_returns([$(($val) as usize),+])
    };
}

fn check_user_range(src: usize, len: usize) -> Result<(), SyscallError> {
    let range_end = src
        .checked_add(len)
        .ok_or(SyscallError::InvalidArg(format!(
            "Read len 0x{len:x}, too large for src: 0x{src:x}. Overflows"
        )))?;
    if src >= USER_VA_START && range_end <= USER_VA_END {
        Ok(())
    } else {
        Err(SyscallError::Failed(format!(
            "Copy overflows user VA range: {src}, {len}"
        )))
    }
}

pub fn copy_from_user(dst: &mut [u8], user_src: usize) -> Result<(), SyscallError> {
    check_user_range(user_src, dst.len())?; // must be inside USER_VA_START..USER_VA_END
    unsafe {
        core::arch::asm!("csrs sstatus, {}", in(reg) 1usize << 18);
        core::ptr::copy_nonoverlapping(user_src as *const u8, dst.as_mut_ptr(), dst.len());
        core::arch::asm!("csrc sstatus, {}", in(reg) 1usize << 18);
    }
    Ok(())
}

pub fn copy_to_user(src: &[u8], user_dst: usize) -> Result<(), SyscallError> {
    check_user_range(user_dst, src.len())?; // must be inside USER_VA_START..USER_VA_END
    unsafe {
        core::arch::asm!("csrs sstatus, {}", in(reg) 1usize << 18);
        core::ptr::copy_nonoverlapping(src.as_ptr(), user_dst as *mut u8, src.len());
        core::arch::asm!("csrc sstatus, {}", in(reg) 1usize << 18);
    }
    Ok(())
}
