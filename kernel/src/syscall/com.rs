use crate::syscall::{SyscallArgs, error::SyscallError, op::SysOp};

pub enum ComOp {
    Pipe,
    Socket,
    Shmget,
    Semget,
    Msgget,
}

impl SysOp for ComOp {
    fn call(&self, ktx: &mut super::context::KernelContext, args: SyscallArgs) {}
}

impl TryFrom<usize> for ComOp {
    type Error = SyscallError;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Pipe,
            1 => Self::Socket,
            2 => Self::Msgget,
            4 => Self::Semget,
            5 => Self::Msgget,
            _ => return Err(SyscallError::InvalidOperation("ComOp", value)),
        })
    }
}

fn pipe(arg: usize) {}
