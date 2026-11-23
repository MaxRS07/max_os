use abi::op::ProcessOp;
use alloc::{format, string::ToString};
use log::error;
use sync::mutex;

use crate::{
    sched::process::Process,
    syscall::{
        error::SyscallError,
        op::{SysOp, exit, str_from_args},
    },
    write_return,
};

impl SysOp for ProcessOp {
    fn call(
        &self,
        ktx: &mut super::context::KernelContext,
        args: super::SyscallArgs,
    ) -> Result<(), SyscallError> {
        match self {
            Self::CurrentPid => {
                let pid = ktx.process()?.pid();
                write_return!(ktx, pid);
                Ok(())
            }
            Self::Spawn => {
                let (name_ptr, len, ..) = args;
                let name = unsafe { str_from_args(name_ptr, len) }?;
                match Process::spawn(name.as_str()) {
                    Ok(pid) => {
                        // exit 0 with value write
                        write_return!(ktx, pid);
                        Ok(())
                    }
                    Err(msg) => Err(SyscallError::Failed(msg.to_string())),
                }
            }
            Self::Fork => {
                let (name_ptr, name_len, entry, ..) = args;
                let name = unsafe { str_from_args(name_ptr, name_len)? };
                let pid = ktx
                    .process()?
                    .fork(&name, entry)
                    .map_err(|error| SyscallError::Failed(error.to_string()))?;
                write_return!(ktx, pid);
                Ok(())
            }
            Self::Kill => {
                let (pid, ..) = args;
                if pid == 0 {
                    return Err(SyscallError::InvalidArg(format!(
                        "Cannot kill kernel process"
                    )));
                }
                ktx.remove_process(pid as u32)?;
                Ok(())
            }
            Self::Wait => todo!(),
        }
    }
}
