use core::{alloc::Layout, ptr::NonNull};

use mem::{align::aligned_up, page_table::page_alloc::Pager};

pub const THREAD_STACK_SIZE: usize = 0x3000;

const KSTACK_SIZE: usize = 0x3000; // 12 KiB
const STACK_ALIGN: usize = 16; // RISC-V ABI
const MAGIC: u32 = 0xA55_FACE;

unsafe extern "C" {
    static _tdata_start: u8;
    static _tdata_end: u8;
    static _tbss_start: u8;
    static _tbss_end: u8;
}

#[derive(Debug)]
pub struct KernelStack {
    base: NonNull<u8>,
    layout: Layout,
    tls_len: usize,
}

impl KernelStack {
    pub fn new(base: *mut u8, layout: Layout, tls_len: usize) -> Self {
        Self {
            base: NonNull::new(base).unwrap(),
            layout,
            tls_len,
        }
    }
    pub fn alloc() -> Option<Self> {
        let tdata = (&raw const _tdata_end).addr() - (&raw const _tdata_start).addr();
        let tbss = (&raw const _tbss_end).addr() - (&raw const _tbss_start).addr();
        let tls_len = aligned_up(tdata + tbss, STACK_ALIGN);
        let size = tls_len + STACK_ALIGN + KSTACK_SIZE;
        let layout = Layout::from_size_align(size, STACK_ALIGN).ok()?;

        let base = NonNull::new(unsafe { alloc::alloc::alloc(layout) })?;
        unsafe {
            let p = base.as_ptr();
            // .tdata is copied from the image, .tbss is zeroed
            core::ptr::copy_nonoverlapping(&raw const _tdata_start, p, tdata);
            core::ptr::write_bytes(p.add(tdata), 0, tls_len - tdata);
            (p.add(tls_len) as *mut u32).write(MAGIC);
        }
        Some(Self {
            base,
            layout,
            tls_len,
        })
    }
    /// the top of the stack region
    pub fn top(&self) -> usize {
        aligned_up(self.base.addr().get() + self.layout.size(), 16)
    }
    /// pointer to the start of the tls region
    pub fn tls(&self) -> *mut u8 {
        self.base.as_ptr()
    }
    /// returns the start address of the guard page
    pub fn guard(&self) -> usize {
        self.base.addr().get() - 0x1000
    }

    pub fn free(&self, page_allocator: &mut dyn Pager) {
        for offset in 0..self.layout.size().div_ceil(0x1000) {
            let ptr = unsafe { self.tls().add(0x1000 * offset) };
            page_allocator.free(ptr);
        }
        page_allocator.free(self.guard() as *mut u8);
    }
}
