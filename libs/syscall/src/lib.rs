#![no_std]

pub mod dev;
pub mod process;
pub mod raw;
pub mod thread;

pub use abi::error::Errno;

pub const CALL_SHIFT: usize = 56;
pub const CALL_MASK: usize = 0xFF << CALL_SHIFT;
pub const OP_MASK: usize = !CALL_MASK;

#[macro_export]
macro_rules! syscall {
    ($id:expr, $op:expr $(,)?) => {
        $crate::raw::raw_syscall(0, 0, 0, 0, 0, 0, $crate::raw::syscall_id($id, $op))
    };
    ($id:expr, $op:expr, $a0:expr $(,)?) => {
        $crate::raw::raw_syscall($a0, 0, 0, 0, 0, 0, $crate::raw::syscall_id($id, $op))
    };
    ($id:expr, $op:expr, $a0:expr, $a1:expr $(,)?) => {
        $crate::raw::raw_syscall($a0, $a1, 0, 0, 0, 0, $crate::raw::syscall_id($id, $op))
    };
    ($id:expr, $op:expr, $a0:expr, $a1:expr, $a2:expr $(,)?) => {
        $crate::raw::raw_syscall($a0, $a1, $a2, 0, 0, 0, $crate::raw::syscall_id($id, $op))
    };
    ($id:expr, $op:expr, $a0:expr, $a1:expr, $a2:expr, $a3:expr $(,)?) => {
        $crate::raw::raw_syscall($a0, $a1, $a2, $a3, 0, 0, $crate::raw::syscall_id($id, $op))
    };
    ($id:expr, $op:expr, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr $(,)?) => {
        $crate::raw::raw_syscall(
            $a0,
            $a1,
            $a2,
            $a3,
            $a4,
            0,
            $crate::raw::syscall_id($id, $op),
        )
    };
    ($id:expr, $op:expr, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr $(,)?) => {
        $crate::raw::raw_syscall(
            $a0,
            $a1,
            $a2,
            $a3,
            $a4,
            $a5,
            $crate::raw::syscall_id($id, $op),
        )
    };
}
