use crate::{
    drivers::power::sys_poweroff,
    syscall::{device::DevOp::Poweroff, error::SyscallError, op::SysOp},
};

/// Device operation syscalls
pub enum DevOp {
    Poweroff,
}

impl SysOp for DevOp {
    fn call(&self, ktx: &mut super::context::KernelContext, args: super::SyscallArgs) {
        match self {
            Poweroff => sys_poweroff(),
        }
    }
}

impl TryFrom<usize> for DevOp {
    type Error = SyscallError;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Poweroff,
            _ => return Err(SyscallError::InvalidOperation("DevOp", value)),
        })
    }
}
