use core::arch::asm;

use crate::{
    arch::riscv::csr::Csr::{MEDELEG, MEPC, MIDELEG, MSTATUS, PMPADDR0, PMPCFG0, SATP},
    drivers::uart::write_char,
    kernel_main,
};

const MPP_MASK: usize = 3 << 11;
const MPP_BIT: usize = 1 << 11;

pub fn enable_user_mode(hart_id: usize, fdt_ptr: *const u8) {
    unsafe {
        let mut status = MSTATUS.read();
        status &= !MPP_MASK;
        status |= MPP_BIT;
        MSTATUS.write(status);

        MEPC.write(kernel_main as *const () as usize);

        let pmpaddr0: usize = !0;
        let pmpcfg0: usize = 0x1F;

        PMPADDR0.write(pmpaddr0);
        PMPCFG0.write(pmpcfg0);

        SATP.write(0);

        let mideleg = 0x222;
        MIDELEG.write(mideleg);

        let medeleg = 0xB000;
        MEDELEG.write(medeleg);

        // bind args to kernel entry
        asm!(
            "mret",
            in("a0") hart_id,
            in("a1") fdt_ptr,
            options(noreturn)
        );
    }
}
