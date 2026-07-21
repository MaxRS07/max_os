use mmio::mmio_write;

use crate::{
    QUEUE_SIZE,
    types::queue::{VirtQueue, VirtqAvail, VirtqDesc, VirtqUsed},
};

pub struct DescBuf {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
}

pub fn send_command_raw(mmio_addr: usize, queue: &mut VirtQueue, bufs: &[DescBuf]) {
    assert!(bufs.len() <= QUEUE_SIZE);
    unsafe {
        let d = queue.desc.as_mut();
        let avail = queue.avail.as_mut();
        let used = queue.used.as_mut();

        for (i, buf) in bufs.iter().enumerate() {
            let is_last = i == bufs.len() - 1;
            d[i].addr = buf.addr;
            d[i].len = buf.len;
            d[i].flags = if is_last { buf.flags } else { buf.flags | 0x1 };
            d[i].next = if is_last { 0 } else { (i + 1) as u16 };
        }

        let last = core::ptr::read_volatile(&used.idx);
        let idx = avail.idx as usize % QUEUE_SIZE;
        avail.ring[idx] = 0;
        avail.idx = avail.idx.wrapping_add(1);

        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        mmio_write::<u32>(mmio_addr, 0x50, 0);

        while core::ptr::read_volatile(&used.idx) == last {
            core::hint::spin_loop();
        }
    }
}

// keep the original as a convenience wrapper
pub fn send_command<Req, Resp>(
    mmio_addr: usize,
    queue: &mut VirtQueue,
    request: &Req,
    response: &mut Resp,
) {
    send_command_raw(
        mmio_addr,
        queue,
        &[
            DescBuf {
                addr: request as *const Req as u64,
                len: size_of::<Req>() as u32,
                flags: 0x0,
            },
            DescBuf {
                addr: response as *mut Resp as u64,
                len: size_of::<Resp>() as u32,
                flags: 0x2,
            },
        ],
    );
}
