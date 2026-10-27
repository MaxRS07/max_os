use core::{alloc::Layout, ptr::NonNull};

use mem::{align::aligned_up, page_table::page_alloc::Pager};

pub const THREAD_STACK_SIZE: usize = 0x3000;

const KSTACK_SIZE: usize = 0x3000; // 12 KiB
const STACK_ALIGN: usize = 16; // RISC-V ABI
const MAGIC: u32 = 0xBEE_A55;

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

/// Allocates memory and assigns region addresses to self
fn map_kernel_stack(&mut self) -> Result<(), ThreadError> {
    // Calculte TSS size using pointers, then copy it to start pointer
    unsafe {
        // linker symbols, use &raw const to load addr, not value
        let tdata_start = &raw const _tdata_start as usize;
        let tdata_end = &raw const _tdata_end as usize;
        let tbss_start = &raw const _tbss_start as usize;
        let tbss_end = &raw const _tbss_end as usize;

        let glob_tss_ptr = tdata_start as *const u8;
        let tbss_len = tbss_end - tbss_start;
        self.tls_len = tdata_end - tdata_start + tbss_len;

        let thread_layout = Layout::from_size_align(self.tls_len + THREAD_STACK_SIZE, 8).unwrap();
        let tls_mut = alloc::alloc::alloc(thread_layout).as_mut().unwrap();

        // copy data from global tls to local space
        core::ptr::copy_nonoverlapping(glob_tss_ptr, tls_mut, self.tls_len);
        self.tls_start = tls_mut;

        // set bounds for a 12KiB stack
        self.stack_top = self.tls_start.add(self.tls_len);
        self.stack_bottom = self.tls_start.add(self.tls_len).add(THREAD_STACK_SIZE);

        self.total_size = THREAD_STACK_SIZE + self.tls_len;
    }

    Ok(())
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
        let size = tls_len + STACK_ALIGN + KSTACK_SIZE; // TLS + canary slot + stack
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
