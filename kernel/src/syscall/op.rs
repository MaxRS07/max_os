use crate::syscall::{SyscallArgs, context::KernelContext};

pub(crate) trait SysOp {
    fn call(&self, ktx: KernelContext, args: SyscallArgs);
}
