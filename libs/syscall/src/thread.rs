use abi::{alloc::boxed::Box, error::Errno};

use crate::syscall;

pub const THREAD_ID: usize = 0;

pub const SPAWN_OP: usize = 0;
pub const YIELD_OP: usize = 1;
pub const CANCEL_OP: usize = 2;
pub const EXIT_OP: usize = 3;

pub fn tspawn<E: FnOnce() + Send + 'static>(name: &str, f: E) -> Result<usize, Errno> {
    let name_ptr = name.as_ptr() as usize;
    let name_len = name.len();
    let arg = Box::into_raw(Box::new(f)) as usize;
    let entry = trampoline::<E> as usize;

    unsafe { syscall!(THREAD_ID, SPAWN_OP, name_ptr, name_len, entry) }
}

pub fn exit(code: usize) -> ! {
    unsafe {
        syscall!(THREAD_ID, EXIT_OP, code);
    }
    loop {}
}

extern "C" fn trampoline<F: FnOnce()>(arg: usize) -> ! {
    let f = unsafe { Box::from_raw(arg as *mut F) };
    f();
    exit(0)
}
