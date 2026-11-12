.global get_raw_time
get_raw_time:
1:  rdtime a1   # read 64bit time into a1
    ret # return usize a1 # return as usize 64 bit