use crate::mm::map::MemoryMap;

pub struct PciAllocator {
    non_pref_next: usize, // next free address
    non_pref_end: usize,  // end of the region
    pref_next: usize,
    pref_end: usize,
}

impl PciAllocator {
    pub fn new(map: MemoryMap) -> Self {
        Self {
            non_pref_next: map.pci_mmio_non_pref.base_address,
            non_pref_end: map.pci_mmio_non_pref.base_address + map.pci_mmio_non_pref.size,
            pref_next: map.pci_mmio_pref.base_address,
            pref_end: map.pci_mmio_pref.base_address + map.pci_mmio_pref.size,
        }
    }

    pub fn alloc_non_pref(&mut self, size: usize) -> Option<usize> {
        // BARs must be aligned to their own size
        // e.g. a 4KB BAR must be at a 4KB-aligned address
        let addr = align_up(self.non_pref_next, size);
        if addr + size > self.non_pref_end {
            return None; // out of space
        }
        self.non_pref_next = addr + size;
        Some(addr)
    }

    pub fn alloc_pref(&mut self, size: usize) -> Option<usize> {
        let addr = align_up(self.pref_next, size);
        if addr + size > self.pref_end {
            return None;
        }
        self.pref_next = addr + size;
        Some(addr)
    }
}

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
