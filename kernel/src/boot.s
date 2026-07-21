.section .text._start
.globl _start
_start:
    // Enable timer for S mode, we are on M mode so it doesnt matter
    li t0, 0x2
    csrs mcounteren, t0

    la sp, _boot_stack_top
    call boot_entry
1:  wfi
    j 1b