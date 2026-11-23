use abi::op::DevOp;

use crate::{
    drivers::power::sys_poweroff,
    syscall::{
        device::DevOp::{Poweroff, Test},
        error::SyscallError,
        op::SysOp,
    },
};

impl SysOp for DevOp {
    fn call(
        &self,
        ktx: &mut super::context::KernelContext,
        args: super::SyscallArgs,
    ) -> Result<(), SyscallError> {
        match self {
            Poweroff => Ok({
                log::info!("Shutting down...");
                sys_poweroff()
            }),
            Test => {
                let process = ktx.process()?.pid();
                log::info!("Test called by process with id: {process}");
                Ok(())
            }
        }
    }
}
