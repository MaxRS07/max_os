use crate::{
    drivers::power::sys_poweroff,
    syscall::{
        device::DevOp::{Poweroff, Test},
        error::SyscallError,
        op::SysOp,
    },
};

/// Device operation syscalls
pub enum DevOp {
    Poweroff,
    Test,
}

impl SysOp for DevOp {
    fn call(
        &self,
        ktx: &mut super::context::KernelContext,
        args: super::SyscallArgs,
    ) -> Result<(), SyscallError> {
        match self {
            Poweroff => Ok(sys_poweroff()),
            Test => {
                let process = ktx.process()?.pid();
                log::info!("Test called by process with id: {process}");
                Ok(())
            }
        }
    }
}

impl TryFrom<usize> for DevOp {
    type Error = SyscallError;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Poweroff,
            1 => Test,
            _ => return Err(SyscallError::InvalidOperation("DevOp", value)),
        })
    }
}
