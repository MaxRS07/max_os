use alloc::{
    boxed::Box,
    vec::{self, Vec},
};
use log::warn;
use mmio::mmio_read;
use net::device::NetDevice;
use sdt::fdt::FDT;
use virtio::{
    ALL_FEATURES,
    init::init_state,
    types::{device::verify_virtio_magic, queue::VirtQueue},
};

use crate::drivers::{
    block::virtio::cfg::VIRTIO_BLK_CFG_OFFSET,
    net::virtio::{
        cfg::VirtioNetConfig,
        cmd::{
            VIRTIO_NET_F_CSUM, VIRTIO_NET_F_CTRL_MAC_ADDR, VIRTIO_NET_F_CTRL_RX,
            VIRTIO_NET_F_CTRL_VLAN, VIRTIO_NET_F_CTRL_VQ, VIRTIO_NET_F_GUEST_ANNOUNCE,
            VIRTIO_NET_F_GUEST_CSUM, VIRTIO_NET_F_GUEST_ECN, VIRTIO_NET_F_GUEST_TSO4,
            VIRTIO_NET_F_GUEST_TSO6, VIRTIO_NET_F_GUEST_UFO, VIRTIO_NET_F_HOST_ECN,
            VIRTIO_NET_F_HOST_TSO4, VIRTIO_NET_F_HOST_TSO6, VIRTIO_NET_F_HOST_UFO, VIRTIO_NET_F_MQ,
            VIRTIO_NET_F_RSC_EXT,
        },
    },
};

pub(crate) mod cfg;
pub(crate) mod cmd;

pub struct VirtioNet {
    config: VirtioNetConfig,
    /// between 1 and 65535 pairs
    max_virtqueue_pairs: u16,
    // N extra transmit queues (N=1 if VIRTIO_NET_F_MQ is not negotiated, otherwise N is set by max_virtqueue_pairs.). These i thing happen in parallel
    /// `max_virtqueue_pairs` entries
    receiveqs: Vec<VirtQueue>,
    /// `max_virtqueue_pairs` entries
    transmitqs: Vec<VirtQueue>,
    /// 2(`max_virtqueue_pairs`) entries
    /// only exists if VIRTIO_NET_F_CTRL_VQ set.
    controlq: Option<VirtQueue>,
}

impl VirtioNet {
    pub fn from_mmio(fdt: &FDT, mmio_idx: usize, cfg_flags: u64, max_pairs: u16) -> Option<Self> {
        if let Err(error) = Self::verify_cfg_flags(cfg_flags) {
            warn!("Failed to initalize NET driver {error}");
            return None;
        }
        let mmio_addr = fdt.virtio_mmio[mmio_idx].base_address;

        if let Err(error) = verify_virtio_magic(mmio_addr) {
            warn!("Failed to initialize NET driver: {error}");
            return None;
        }

        if let Err(error) = init_state(mmio_addr, ALL_FEATURES) {
            warn!("Failed to initialize NET driver: {error}")
        }

        let receiveqs = alloc::vec![VirtQueue::default(); max_pairs as usize];
        let transmitqs = alloc::vec![VirtQueue::default(); max_pairs as usize];

        let config: VirtioNetConfig = mmio_read(mmio_addr, VIRTIO_BLK_CFG_OFFSET);

        let max_virtqueue_pairs = if cfg_flags & VIRTIO_NET_F_MQ != 0 {
            // pairs must be 1-65535
            max_pairs.max(1)
        } else {
            // default to 1 if not specified
            1
        };

        let controlq = if Self::has_flag(cfg_flags, VIRTIO_NET_F_CTRL_VQ) {
            Some(VirtQueue::default())
        } else {
            None
        };

        Some(Self {
            config,
            max_virtqueue_pairs,
            receiveqs,
            transmitqs,
            controlq,
        })
    }
    // fn populate_buffers(cfg_flags: u64, queue: &mut VirtQueue) -> Result<(), ()> {
    //     let min_size = if Self::has_flag(
    //         cfg_flags,
    //         VIRTIO_NET_F_GUEST_UFO | VIRTIO_NET_F_GUEST_TSO4 | VIRTIO_NET_F_GUEST_TSO6,
    //     ) {
    //         0xFFFF
    //     } else {
    //         0x5F6
    //     };
    // }
    /// access a queue reference by index
    fn get_queue(&mut self, index: usize) -> Option<&mut VirtQueue> {
        if index >= (2 * self.max_virtqueue_pairs + 1) as usize {
            return self.controlq.as_mut();
        }
        if index.is_multiple_of(2) {
            self.receiveqs.get_mut(index / 2)
        } else {
            self.transmitqs.get_mut(index / 2 + 1)
        }
    }
    fn verify_cfg_flags(cfg_flags: u64) -> Result<(), &'static str> {
        let has_tsox_guest =
            Self::has_flag(cfg_flags, VIRTIO_NET_F_GUEST_TSO4 | VIRTIO_NET_F_GUEST_TSO6);
        let has_tsox_host =
            Self::has_flag(cfg_flags, VIRTIO_NET_F_HOST_TSO4 | VIRTIO_NET_F_HOST_TSO6);
        let has_guest_csum = Self::has_flag(cfg_flags, VIRTIO_NET_F_GUEST_CSUM);
        let has_csum = Self::has_flag(cfg_flags, VIRTIO_NET_F_CSUM);
        let has_control = Self::has_flag(cfg_flags, VIRTIO_NET_F_CTRL_VQ);

        if has_tsox_guest && !has_guest_csum {
            return Err("VIRTIO_NET_F_GUEST_TSOx Requires VIRTIO_NET_F_GUEST_CSUM.");
        }
        if Self::has_flag(cfg_flags, VIRTIO_NET_F_GUEST_ECN) && !has_tsox_guest {
            return Err(
                "VIRTIO_NET_F_GUEST_ECN Requires VIRTIO_NET_F_GUEST_TSO4 or VIRTIO_NET_F_GUEST_TSO6.",
            );
        }
        if Self::has_flag(cfg_flags, VIRTIO_NET_F_GUEST_UFO) && !has_csum {
            return Err("VIRTIO_NET_F_GUEST_UFO Requires VIRTIO_NET_F_GUEST_CSUM.");
        }
        if has_tsox_host && !has_csum {
            return Err("VIRTIO_NET_F_HOST_TSOX Requires VIRTIO_NET_F_CSUM.");
        }
        if Self::has_flag(cfg_flags, VIRTIO_NET_F_HOST_ECN) && !has_tsox_host {
            return Err(
                "VIRTIO_NET_F_HOST_ECN Requires VIRTIO_NET_F_HOST_TSO4 or VIRTIO_NET_F_HOST_TSO6.",
            );
        }
        if Self::has_flag(cfg_flags, VIRTIO_NET_F_HOST_UFO) && !has_csum {
            return Err("VIRTIO_NET_F_HOST_UFO Requires VIRTIO_NET_F_CSUM.");
        }
        if Self::has_flag(cfg_flags, VIRTIO_NET_F_CTRL_RX) && !has_control {
            return Err("VIRTIO_NET_F_CTRL_RX Requires VIRTIO_NET_F_CTRL_VQ.");
        }
        if Self::has_flag(cfg_flags, VIRTIO_NET_F_CTRL_VLAN) && !has_control {
            return Err("VIRTIO_NET_F_CTRL_VLAN Requires VIRTIO_NET_F_CTRL_VQ.");
        }
        if Self::has_flag(cfg_flags, VIRTIO_NET_F_GUEST_ANNOUNCE) && !has_control {
            return Err("VIRTIO_NET_F_GUEST_ANNOUNCE Requires VIRTIO_NET_F_CTRL_VQ.");
        }
        if Self::has_flag(cfg_flags, VIRTIO_NET_F_MQ) && !has_control {
            return Err("VIRTIO_NET_F_MQ Requires VIRTIO_NET_F_CTRL_VQ.");
        }
        if Self::has_flag(cfg_flags, VIRTIO_NET_F_CTRL_MAC_ADDR) && !has_control {
            return Err("VIRTIO_NET_F_CTRL_MAC_ADDR Requires VIRTIO_NET_F_CTRL_VQ.");
        }
        if Self::has_flag(cfg_flags, VIRTIO_NET_F_RSC_EXT) && !has_tsox_host {
            return Err(
                "VIRTIO_NET_F_RSC_EXT Requires VIRTIO_NET_F_HOST_TSO4 or VIRTIO_NET_F_HOST_TSO6.",
            );
        }
        Ok(())
    }
    fn has_flag(cfg_flags: u64, flag: u64) -> bool {
        cfg_flags & flag != 0
    }
}

impl NetDevice for VirtioNet {
    fn send_packet() {}
}
