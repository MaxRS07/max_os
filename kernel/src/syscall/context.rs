use core::{
    ops::DerefMut,
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

pub type TrapFrame = [usize; 32];

// Context for the syscall operations
/// Kernel context struct
///
/// Using
pub struct KernelContext {
    vfs: Option<MutexGuard<'static, vfs::Vfs<'static>>>,
    process: Option<*mut Process>,
    trap_frame: *mut TrapFrame,
}

impl KernelContext {
    pub fn new(trap_frame: *mut usize) -> Self {
        Self {
            vfs: None,
            process: None,
            trap_frame: trap_frame as *mut TrapFrame,
        }
    }
    pub fn process(&mut self) -> Result<&mut Process, SyscallError> {
        match self.process {
            Some(process) => Ok(unsafe { &mut *process }),
            None => {
                let process = CURRENT_PROCESS.load(Ordering::Acquire);
                if process.is_null() {
                    Err(SyscallError::Busy(format!(
                        "Failed to retrieve running process"
                    )));
                }
                self.process = Some(process);
                Ok(unsafe { &mut *process })
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
    /// This is the trap frame saved before ecall. Writes to this persist after interrupt.
    /// ## Registers
    /// `0`. `ra` \
    /// `1`. `tp` \
    /// `2`. `t0` \
    /// `3`. `t1` \
    /// `4`. `t2` \
    /// `5`. `a0` \
    /// `6`. `a1` \
    /// `7`. `a2` \
    /// `8`. `a3` \
    /// `9`. `a4` \
    /// `10`. `a5` \
    /// `11`. `a6` \
    /// `12`. `a7` \
    /// `13`. `t3` \
    /// `14`. `t4` \
    /// `15`. `t5` \
    /// `16`. `t6` \
    /// `17`. `s0` \
    /// `18`. `s1` \
    /// `19`. `s2` \
    /// `20`. `s3` \
    /// `21`. `s4` \
    /// `22`. `s5` \
    /// `23`. `s6` \
    /// `24`. `s7` \
    /// `25`. `s8` \
    /// `26`. `s9` \
    /// `27`. `s10` \
    /// `28`. `s11` \
    /// `29`. `unused` \
    /// `30`. `unused` \
    /// `31`. `unused` \
    /// `32`. `unused`
    pub fn trap_frame(&mut self) -> &mut TrapFrame {
        unsafe { &mut *self.trap_frame }
    }
}
