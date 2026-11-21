.global user_test_start
.global user_test_end

user_test_start:
    li a6, 0x1 # device test syscall id
    ecall    

    li a6, 0x0400000000000004 # kill process syscall id
    ecall    
    j . # loop net, ecall should 
user_test_end:
