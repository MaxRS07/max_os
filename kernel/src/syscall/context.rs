use core::{
    cell::{RefCell, RefMut},
    ops::DerefMut,
    sync::atomic::Ordering,
    time::Duration,
};

use alloc::{borrow::ToOwned, fmt::format, format};
use fs::vfs::{self, Vfs};
use graphics::display_driver::DisplayDriver;
use log::error;
use sync::mutex_guard::MutexGuard;

use crate::{
    drivers::gpu::GPU_DEVICE,
    fs::GLOBAL_FS,
    sched::{PROCESS_TABLE, process::Process, scheduler::CURRENT_PROCESS},
    syscall::error::SyscallError,
};

pub type TrapFrame = [usize; 32];

/// Context for syscall operations, providing lazy locked global statics owned by the kernel
pub struct KernelContext {
    vfs: RefCell<Option<MutexGuard<'static, vfs::Vfs<'static>>>>,
    gpu: RefCell<Option<MutexGuard<'static, &'static mut dyn DisplayDriver>>>,
    trap_frame: *mut TrapFrame,
}

impl KernelContext {
    pub fn new(trap_frame: *mut usize) -> Self {
        Self {
            vfs: RefCell::new(None),
            gpu: RefCell::new(None),
            trap_frame: trap_frame as *mut TrapFrame,
        }
    }
    pub fn process(&self) -> Result<&mut Process, SyscallError> {
        let process = CURRENT_PROCESS.load(Ordering::Acquire);
        if process.is_null() {
            return Err(SyscallError::Busy(format!(
                "Failed to retrieve running process"
            )));
        }
        Ok(unsafe { &mut *process })
    }
    pub fn get_process(&self, pid: u32) -> Result<&mut Process, SyscallError> {
        match PROCESS_TABLE.wait().lock().get(&pid) {
            Some(process) => Ok(unsafe { &mut *process }),
            None => Err(SyscallError::InvalidArg(
                "Failed to get process, PID invalid".to_owned(),
            )),
        }
    }
    pub fn remove_process(&self, pid: u32) -> Result<*mut Process, SyscallError> {
        PROCESS_TABLE
            .wait()
            .lock()
            .remove(&pid)
            .ok_or(SyscallError::InvalidArg(format!(
                "Failed to kill process, invalid pid: {pid}"
            )))
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
                    .lock_timeout(Duration::from_secs(1))
                    .map_err(|err| {
                        SyscallError::Busy(format!("Failed to retrieve lock on file: {err}"))
                    })?,
            )
        }
        Ok(RefMut::map(slot, |s| &mut **s.as_mut().unwrap()))
    }
    /// get an instance of the active gpu driver
    pub fn gpu(&self) -> Result<RefMut<'_, &'static mut dyn DisplayDriver>, SyscallError> {
        let mut slot = self
            .gpu
            .try_borrow_mut()
            .map_err(|_| SyscallError::Busy("GPU already borrowed in this context".to_owned()))?;
        if slot.is_none() {
            *slot = Some(
                GPU_DEVICE
                    .wait()
                    .lock_timeout(Duration::from_secs(1))
                    .map_err(|err| {
                        SyscallError::Busy(format!(
                            "Failed to retrive the global GPU driver: {err}"
                        ))
                    })?,
            );
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
