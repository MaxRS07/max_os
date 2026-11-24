use abi::error::Errno;

pub(crate) fn syscall(
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
            inlateout("a0") args[0] as isize => code,
            inlateout("a1") args[1] => out,
            in("a2") args[2],
            in("a3") args[3],
            in("a4") args[4],
            in("a5") args[5],
            in("a7") id,
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
