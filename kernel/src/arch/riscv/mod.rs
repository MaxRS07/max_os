use crate::arch::riscv;

pub mod csr;
pub mod interrupt;
pub mod mode;

pub fn setup() {
    interrupt::setup();
}
