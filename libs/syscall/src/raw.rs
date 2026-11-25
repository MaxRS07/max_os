use abi::error::Errno;

use crate::{CALL_MASK, CALL_SHIFT, OP_MASK};

pub const fn syscall_id(id: usize, op: usize) -> usize {
    ((id << CALL_SHIFT) & CALL_MASK) | (op & OP_MASK)
}
pub unsafe fn raw_syscall(
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    id: usize,
) -> Result<usize, Errno> {
    let mut code: isize;
    let mut out: usize;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") arg0 as isize => code,
            inlateout("a1") arg1 => out,
            inlateout("a2") arg2 => _,
            inlateout("a3") arg3 => _,
            inlateout("a4") arg4 => _,
            inlateout("a5") arg5 => _,
            lateout("a6") _,
            inlateout("a7") id => _,
            options(nostack),
        );
    }
    check_call(code, out)
}

fn check_call(code: isize, out: usize) -> Result<usize, Errno> {
    if code < 0 {
        let err = Errno::from_raw(code);
        return Err(err);
    }
    Ok(out)
}
