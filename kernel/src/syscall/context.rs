use core::{
    cell::{RefCell, RefMut},
    ops::DerefMut,
    sync::atomic::Ordering,
};

use alloc::{borrow::ToOwned, format, string::ToString};
use fs::vfs::{self, Vfs};
use log::error;
use sync::{map_guard::MappedMutexGuard, mutex_guard::MutexGuard};

use crate::{
    drivers::time::GLOBAL_TIME,
    fs::GLOBAL_FS,
    sched::{
        PROCESS_TABLE, SCHEDULER,
        process::{Process, ProcessTable},
        scheduler::CURRENT_PROCESS,
    },
    syscall::error::SyscallError,
};

pub type TrapFrame = [usize; 32];

// Context for the syscall operations
/// Kernel context struct
///
/// Using
pub struct KernelContext {
    vfs: RefCell<Option<MutexGuard<'static, vfs::Vfs<'static>>>>,
    trap_frame: *mut TrapFrame,
}

impl KernelContext {
    pub fn new(trap_frame: *mut usize) -> Self {
        Self {
            vfs: RefCell::new(None),
            trap_frame: trap_frame as *mut TrapFrame,
        }
    }
    pub fn process(&self) -> Result<MappedMutexGuard<'_, ProcessTable, Process>, SyscallError> {
        let pid = CURRENT_PROCESS.load(Ordering::Acquire);
        let process_table = PROCESS_TABLE.wait().lock();
        process_table
            .try_map(|table| table.process_mut(pid))
            .map_err(|te| SyscallError::Failed(format!("{te}")))
    }
    pub fn get_process(
        &self,
        pid: u32,
    ) -> Result<MappedMutexGuard<'_, ProcessTable, Process>, SyscallError> {
        let process_table = PROCESS_TABLE.wait().lock();
        process_table
            .try_map(|table| table.process_mut(pid))
            .map_err(|te| SyscallError::Failed(format!("{te}")))
    }
    // Removes and destroys process with id `pid` from the process table and removes its thread ids from the scheduler. Returns the number of freed threads on success
    pub fn remove_process(&self, pid: u32) -> Result<usize, SyscallError> {
        PROCESS_TABLE
            .wait()
            .lock()
            .remove_process(pid)
            .map_err(|err| SyscallError::Failed(err.to_string()))?;
        Ok(SCHEDULER.wait().lock().remove_process_threads(pid))
    }
    pub fn vfs(&self) -> Result<RefMut<'_, Vfs<'static>>, SyscallError> {
        let mut slot = self
            .vfs
            .try_borrow_mut()
            .map_err(|_| SyscallError::Busy("VFS already borrowed in this context".to_owned()))?;
        if slot.is_none() {
            *slot = Some(
                GLOBAL_FS
                    .wait()
                    .lock_timeout(GLOBAL_TIME.wait(), 1000)
                    .map_err(|err| {
                        SyscallError::Busy(format!("Failed to retrieve lock on file: {err}"))
                    })?,
            )
        }
        Ok(RefMut::map(slot, |s| &mut **s.as_mut().unwrap()))
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
    pub fn trap_frame(&self) -> &mut TrapFrame {
        unsafe { &mut *self.trap_frame }
    }
}
