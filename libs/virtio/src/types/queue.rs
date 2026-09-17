use alloc::boxed::Box;

use crate::{QUEUE_SIZE, VIRTIO_MMIO_QUEUE_NOTIFY};

#[repr(C, align(16))]
#[derive(Clone, Copy, Default, Debug)]
pub struct VirtqDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

#[repr(C, align(2))]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtqAvail {
    pub flags: u16,
    pub idx: u16,
    pub ring: [u16; QUEUE_SIZE],
}

#[repr(C, align(4))]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtqUsed {
    pub flags: u16,
    pub idx: u16,
    pub ring: [VirtqUsedElem; QUEUE_SIZE],
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct VirtqUsedElem {
    pub id: u32,
    pub len: u32,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct VirtQueue {
    pub desc: Box<[VirtqDesc; QUEUE_SIZE]>,
    pub avail: Box<VirtqAvail>,
    pub used: Box<VirtqUsed>,
    /// queue index within the device, set by `init_virtqueue`; needed so `notify` can
    /// tell the device which queue changed
    pub queue_idx: u32,
    /// `used.idx` value already consumed by `pop_used`
    pub last_used_idx: u16,
}
pub struct DescBuf {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
}
impl VirtQueue {
    // keep the original as a convenience wrapper
    pub fn push_desc<Req, Resp>(&mut self, mmio_addr: usize, request: &Req, response: &mut Resp) {
        let last = self.push_desc_raw(
            mmio_addr,
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
            0,
        );
        self.wait_used(last);
    }
    /// Submits the descriptor chain but doesnt wait on the write, you must call wait_used manually
    pub fn push_desc_raw(&mut self, mmio_addr: usize, bufs: &[DescBuf], desc_start: u16) -> u16 {
        assert!(
            bufs.len() <= QUEUE_SIZE,
            "buf queue longer than max queue size size, rejecting command"
        );
        unsafe {
            let d = self.desc.as_mut();
            let used = self.used.as_mut();

            for (i, buf) in bufs.iter().enumerate() {
                let is_last = i == bufs.len() - 1;

                let curr_slot = (desc_start as usize + i) % QUEUE_SIZE;
                let next_slot = (desc_start as usize + i + 1) % QUEUE_SIZE;

                core::ptr::write_volatile(&mut d[curr_slot].addr, buf.addr);
                core::ptr::write_volatile(&mut d[curr_slot].len, buf.len);
                core::ptr::write_volatile(
                    &mut d[curr_slot].flags,
                    if is_last { buf.flags } else { buf.flags | 0x1 }, // 0x1 = VIRTIO_DESC_F_NEXT
                );
                core::ptr::write_volatile(
                    &mut d[curr_slot].next,
                    if is_last { 0 } else { next_slot as u16 }, // Map to the next physical slot
                );
            }
            let last = core::ptr::read_volatile(&used.idx);

            self.avail_push(desc_start);

            self.notify(mmio_addr);

            last
        }
    }
    /// Pushes a single struct `T` to the descriptor queue. Increments available queue and noifies the device
    pub fn push_descriptor<T>(&mut self, idx: u16, value: T, flags: u16) {
        let box_t = Box::new(T);
        let t_ptr = Box::into_raw(box_t) as u64;

        self.desc[idx as usize].addr = rx_buffer_addr;
        self.desc[idx as usize].len = size_of::<RxBuffer>() as u32;
        self.desc[idx as usize].flags = flags;
        self.desc[idx as usize].next = 0;

        let avail_slot = queue.avail.idx % 256;
        self.avail.ring[avail_slot as usize] = idx;
        self.avail_push(idx);
    }

    /// spins until the device has processed the descriptor chain and changes `used.idx`
    pub fn wait_used(&self, last: u16) {
        unsafe {
            while core::ptr::read_volatile(&self.used.idx) == last {
                core::hint::spin_loop();
            }

            core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire);
        }
    }

    /// Pushes a descriptor chain head onto the avail ring, making it visible to the device.
    pub fn avail_push(&mut self, desc_idx: u16) {
        unsafe {
            let avail = self.avail.as_mut();
            let slot = avail.idx as usize % QUEUE_SIZE;
            core::ptr::write_volatile(&mut avail.ring[slot], desc_idx);

            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            core::ptr::write_volatile(&mut avail.idx, avail.idx.wrapping_add(1));
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        }
    }

    /// Tells the device that this queue has new avail entries.
    pub fn notify(&self, mmio_addr: usize) {
        mmio::mmio_write::<u32>(mmio_addr, VIRTIO_MMIO_QUEUE_NOTIFY, self.queue_idx);
    }

    /// Reads the `len` written back by the block driver
    pub fn used_len_for(&self, idx: u16) -> u32 {
        unsafe { core::ptr::read_volatile(&self.used.ring[idx as usize % QUEUE_SIZE]).len }
    }

    /// Pops the next unconsumed used-ring entry
    pub fn pop_used(&mut self) -> Option<(u16, u32)> {
        unsafe {
            let used = self.used.as_ref();
            if self.last_used_idx == core::ptr::read_volatile(&used.idx) {
                return None;
            }
            let slot = self.last_used_idx as usize % QUEUE_SIZE;
            let elem = core::ptr::read_volatile(&used.ring[slot]);
            self.last_used_idx = self.last_used_idx.wrapping_add(1);
            Some((elem.id as u16, elem.len))
        }
    }
}

impl Default for VirtQueue {
    fn default() -> Self {
        Self {
            desc: Box::new([VirtqDesc::default(); QUEUE_SIZE]),
            avail: Box::new(VirtqAvail::default()),
            used: Box::new(VirtqUsed::default()),
            queue_idx: 0,
            last_used_idx: 0,
        }
    }
}
