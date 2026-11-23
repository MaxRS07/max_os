use abi::op::ComOp;

use crate::syscall::{SyscallArgs, error::SyscallError, op::SysOp};

impl SysOp for ComOp {
    fn call(
        &self,
        ktx: &mut super::context::KernelContext,
        args: SyscallArgs,
    ) -> Result<(), SyscallError> {
        Ok(())
    }
}

fn pipe(arg: usize) {}
