.section .text.user_test, "ax"
.align 2
.global s_trap_entry
s_trap_entry:
csrrw sp, sscratch, sp  # Swap stack pointer with sscratch
bnez sp, 1f
csrrw sp, sscratch, sp
csrw sscratch, sp
sd sscratch, 248(sp)  # Save sscratch to stack
1:  addi sp, sp, -256
    sd ra,   0(sp)
    sd tp,   8(sp)
    sd t0,  16(sp)
    sd t1,  24(sp)
    sd t2,  32(sp)
    sd a0,  40(sp)
    mv a0,  sp  # Pass TrapFrame pointer in a0
    sd a1,  48(sp)
    sd a2,  56(sp)
    sd a3,  64(sp)
    sd a4,  72(sp)
    sd a5,  80(sp)
    sd a6,  88(sp)
    sd a7,  96(sp)
    sd t3,  104(sp)
    sd t4,  112(sp)
    sd t5,  120(sp)
    sd t6,  128(sp)
    sd s0,  136(sp)
    sd s1,  144(sp)
    sd s2,  152(sp)
    sd s3,  160(sp)
    sd s4,  168(sp)
    sd s5,  176(sp)
    sd s6,  184(sp)
    sd s7,  192(sp)
    sd s8,  200(sp)
    sd s9,  208(sp)
    sd s10, 216(sp)
    sd s11, 224(sp)
    # Save CSRs
    csrr t0, sepc
    sd t0,  232(sp)
    csrr t0, sstatus
    sd t0,  240(sp)
    csrr t0, sscratch
    sd t0,  248(sp)
    csrw sscratch, x0
    # Call actual handler
    call s_trap_handler
    # Restore CSRs
    ld t0,  232(sp)
    csrw sepc, t0
    ld t0,  240(sp)
    csrw sstatus, t0
    andi t0, t0, 0x100
    bnez t0, 2f
    addi t0, sp, 256
    csrw sscratch, t0
2:  # Restore stack pointer
    # Load back saved registers
    ld ra,   0(sp)
    ld tp,   8(sp)
    ld t0,  16(sp)
    ld t1,  24(sp)
    ld t2,  32(sp)
    ld a0,  40(sp)
    ld a1,  48(sp)
    ld a2,  56(sp)
    ld a3,  64(sp)
    ld a4,  72(sp)
    ld a5,  80(sp)
    ld a6,  88(sp)
    ld a7,  96(sp)
    ld t3,  104(sp)
    ld t4,  112(sp)
    ld t5,  120(sp)
    ld t6,  128(sp)
    ld s0,  136(sp)
    ld s1,  144(sp)
    ld s2,  152(sp)
    ld s3,  160(sp)
    ld s4,  168(sp)
    ld s5,  176(sp)
    ld s6,  184(sp)
    ld s7,  192(sp)
    ld s8,  200(sp)
    ld s9,  208(sp)
    ld s10, 216(sp)
    ld s11, 224(sp)
    # Reset stack pointer back
    ld sp, 248(sp)
    # Jump to next instruction
    sret