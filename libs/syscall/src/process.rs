use abi::error::Errno;

use crate::syscall;

pub const PROCESS_ID: usize = 4;

pub const CURRENT_PID_OP: usize = 0;
pub const SPAWN_OP: usize = 1;
pub const FORK_OP: usize = 2;
pub const WAIT_OP: usize = 3;
pub const KILL_OP: usize = 4;

/// returns the PID of the current running process
pub fn ppid() -> Result<usize, Errno> {
    unsafe { syscall!(PROCESS_ID, CURRENT_PID_OP) }
}
/// Creates a new blank process. Returns the PID of the new thread
pub fn pspawn(name: &str) -> Result<usize, Errno> {
    let name_ptr = name.as_ptr();
    let name_len = name.len();
    unsafe { syscall!(PROCESS_ID, SPAWN_OP, name_ptr as usize, name_len) }
}
/// forks the current running thread, copying memory over. Returns the PID of the fork thread.
pub fn pfork_name(name: &str) -> Result<usize, Errno> {
    let name_ptr = name.as_ptr();
    let name_len = name.len();
    unsafe { syscall!(PROCESS_ID, FORK_OP, name_ptr as usize, name_len) }
}
pub fn pfork() -> Result<usize, Errno> {
    unsafe { syscall!(PROCESS_ID, FORK_OP, 0, 0) }
}
/// kills the process with the given PID
pub fn pkill(pid: usize) -> Result<(), Errno> {
    unsafe {
        syscall!(PROCESS_ID, KILL_OP, pid)?;
    }
    Ok(())
}
