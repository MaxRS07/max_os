.section .text, "ax"
.align 2

.global user_test_start
.global user_test_end

user_test_start:
    li a7, 0x01 # device test syscall id
    ecall    

    li a7, 0x0400000000000000 # current PID
    ecall
    mv a0, a1 # move PID to a0
    li a7, 0x0400000000000004 # kill this process (pid in a0)
    ecall    
    j . # loop net, ecall should 
user_test_end:
