use core::ops::{Index, IndexMut};

#[repr(C)]
pub struct TrapFrame {
    pub ra: usize, // 0
    pub tp: usize, // 8
    pub t0: usize, // 16
    pub t1: usize,
    pub t2: usize,
    pub a: [usize; 8], // 40..104   a0?a7, so args are a[0..6], id is a[6]
    pub t3: usize,     // 104
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
    pub s: [usize; 12], // 136..232  s0?s11
    pub sepc: usize,    // 232
    pub sstatus: usize, // 240
    pub sp: usize,      // 248       interrupted sp (user or kernel)
}
impl TrapFrame {
    pub fn syscall_id(&self) -> usize {
        self.a[7]
    }
    pub fn args(&self) -> [usize; 6] {
        let a = &self.a;
        [a[0], a[1], a[2], a[3], a[4], a[5]]
    }
    pub fn set_status(&mut self, status: isize) {
        self.a[0] = status as usize;
    }
    pub fn set_returns<const N: usize>(&mut self, vals: [usize; N]) {
        const { assert!(N <= 7, "at most 7 return values (a1-a7)") };
        self.a[1..=N].copy_from_slice(&vals);
    }
}
const _: () = {
    use core::mem::{offset_of, size_of};
    assert!(size_of::<TrapFrame>() == 256);
    assert!(offset_of!(TrapFrame, a) == 40);
    assert!(offset_of!(TrapFrame, s) == 136);
    assert!(offset_of!(TrapFrame, sepc) == 232);
    assert!(offset_of!(TrapFrame, sstatus) == 240);
    assert!(offset_of!(TrapFrame, sp) == 248);
};
