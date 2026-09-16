use crate::syscall::{SyscallArgs, context::KernelContext};

pub(crate) trait SysOp {
    fn call(&mut self, ktx: KernelContext, args: SyscallArgs);
}
