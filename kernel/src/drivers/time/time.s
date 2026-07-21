.global get_raw_time
get_raw_time:
1:  rdtimeh a1   # upper
    rdtime  a0   # lower
    rdtimeh t0   # read upper again
    bne a1, t0, 1b # go to 1 if upper changed (lower flowed over)
    ret # return u64 (a1 << 32) | a0)