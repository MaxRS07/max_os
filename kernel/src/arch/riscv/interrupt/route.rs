use core::{
    arch::asm,
    sync::atomic::{Ordering, fence},
};

use crate::arch::riscv::csr::Csr::SCAUSE;

const CAUSE_MASK: usize = 1 << (usize::BITS - 1);
const EXCEPTION_MASK: usize = !CAUSE_MASK;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Trap {
    Exception(ExceptionCode),
    Interrupt(InterruptCode),
}

#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum InterruptCode {
    UserSoftware = 0,
    SupervisorSoftware = 1,
    VSSoftware = 2,
    MachineSoftware = 3,
    UserTimer = 4,
    SupervisorTimer = 5,
    VSTimer = 6,
    MachineTimer = 7,
    UserExternal = 8,
    SupervisorExternal = 9,
    VSExternal = 10,
    MachineExternal = 11,
    SupervisorGuestExternal = 12,
    CounterOverflow = 13,
    LowRASEvent = 35,
    HighRASEvent = 43,
    Generic,
    Unknown(usize),
}

impl From<usize> for InterruptCode {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::UserSoftware,
            1 => Self::SupervisorSoftware,
            2 => Self::VSSoftware,
            3 => Self::MachineSoftware,
            4 => Self::UserTimer,
            5 => Self::SupervisorTimer,
            6 => Self::VSTimer,
            7 => Self::MachineTimer,
            8 => Self::UserExternal,
            9 => Self::SupervisorExternal,
            10 => Self::VSExternal,
            11 => Self::MachineExternal,
            12 => Self::SupervisorGuestExternal,
            13 => Self::CounterOverflow,
            35 => Self::LowRASEvent,
            43 => Self::HighRASEvent,
            value => Self::Unknown(value),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ExceptionCode {
    InstructionAddressMisaligned = 0,
    InstructionAccessFault = 1,
    IllegalInstruction = 2,
    Breakpoint = 3,
    LoadAddressMisaligned = 4,
    LoadAccessFault = 5,
    StoreAddressMisaligned = 6,
    StoreAccessFault = 7,
    EnvCallFromUMode = 8,
    EnvCallFromSMode = 9,
    EnvCallFromMMode = 11,
    InstructionPageFault = 12,
    LoadPageFault = 13,
    StorePageFault = 15,
    Unknown(usize),
}

impl From<usize> for ExceptionCode {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::InstructionAddressMisaligned,
            1 => Self::InstructionAccessFault,
            2 => Self::IllegalInstruction,
            3 => Self::Breakpoint,
            4 => Self::LoadAddressMisaligned,
            5 => Self::LoadAccessFault,
            6 => Self::StoreAddressMisaligned,
            7 => Self::StoreAccessFault,
            8 => Self::EnvCallFromUMode,
            9 => Self::EnvCallFromSMode,
            11 => Self::EnvCallFromMMode,
            12 => Self::InstructionPageFault,
            13 => Self::LoadPageFault,
            15 => Self::StorePageFault,
            value => Self::Unknown(value),
        }
    }
}

/// Parses the trap type from the mcause register
/// # Safety
pub unsafe fn parse_scause() -> Trap {
    let scause = unsafe { SCAUSE.read() };
    fence(Ordering::SeqCst);
    let reason = scause & CAUSE_MASK; // signficant bit, 1 = interrupt, 0 = exception
    let code = scause & EXCEPTION_MASK; // select the lower bytes for code
    match reason {
        0 => Trap::Exception(ExceptionCode::from(code)),
        _ => Trap::Interrupt(InterruptCode::from(code)),
    }
}
