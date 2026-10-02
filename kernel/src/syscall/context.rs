use core::{
    ops::DerefMut,
    process,
    ptr::{null, null_mut},
    sync::atomic::Ordering,
};

use alloc::format;
use fs::vfs::{self, Vfs};
use log::error;
use sync::mutex_guard::MutexGuard;

use crate::{
    drivers::time::GLOBAL_TIME,
    fs::GLOBAL_FS,
    sched::{
        PROCESS_TABLE, SCHEDULER, error::ThreadError, process::Process, scheduler::CURRENT_PROCESS,
    },
    syscall::error::SyscallError,
};

// Context for the syscall operations
/// Kernel context struct
///
/// Using
pub struct KernelContext {
    vfs: Option<MutexGuard<'static, vfs::Vfs<'static>>>,
    process: Option<*mut Process>,
}

impl KernelContext {
    pub fn new() -> Self {
        Self {
            vfs: None,
            process: None,
        }
    }
    pub fn process(&mut self) -> Option<&mut Process> {
        match self.process {
            Some(process) => Some(unsafe { &mut *process }),
            None => {
                let process = CURRENT_PROCESS.load(Ordering::Acquire);
                if process.is_null() {
                    error!("Process is undefined");
                    return None;
                }
                self.process = Some(process);
                Some(unsafe { &mut *process })
            }
        }
    }
    pub fn vfs(&mut self) -> Result<&mut Vfs<'static>, SyscallError> {
        if self.vfs.is_none() {
            let vfs = GLOBAL_FS
                .wait()
                .lock_timeout(GLOBAL_TIME.wait(), 1000)
                .map_err(|err| {
                    SyscallError::Busy(format!("Failed to retrieve lock on file: {err}"))
                })?;
            self.vfs = Some(vfs);
        }
        Ok(self.vfs.as_mut().unwrap().deref_mut())
    }
}
