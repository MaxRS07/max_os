.section .text._start
.globl _start
_start:
    la sp, _stack_top
    call kernel
1:  wfi
    j 1b