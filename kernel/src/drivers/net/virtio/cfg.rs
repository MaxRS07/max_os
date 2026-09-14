use alloc::vec::Vec;

const VIRTIO_NET_S_LINK_UP: u8 = 1;
const VIRTIO_NET_S_ANNOUNCE: u8 = 2;
/// Net device config
pub struct VirtioNetConfig {
    pub mac: [u8; 6],
    pub status: u16,
    pub max_virtqueue_pairs: u16,
    pub mtu: u16,
}

const VIRTIO_NET_CTRL_RX: u64 = 0;
const VIRTIO_NET_CTRL_RX_PROMISC: u64 = 0;
const VIRTIO_NET_CTRL_RX_ALLMULTI: u64 = 1;
const VIRTIO_NET_CTRL_RX_ALLUNI: u64 = 2;
const VIRTIO_NET_CTRL_RX_NOMULTI: u64 = 3;
const VIRTIO_NET_CTRL_RX_NOUNI: u64 = 4;
const VIRTIO_NET_CTRL_RX_NOBCAST: u64 = 5;

/* ack values */
const VIRTIO_NET_OK: u8 = 0;
const VIRTIO_NET_ERR: u8 = 1;
struct VirtioNetCtrl {
    class: u8,
    command: u8,
    command_specific_data: Vec<u8>,
    ack: u8,
}

struct VirtioNetCtrlMac {
    entries: u32,
    macs: Vec<[u8; 6]>,
}

const VIRTIO_NET_CTRL_MAC: u8 = 1;
const VIRTIO_NET_CTRL_MAC_TABLE_SET: u8 = 0;
const VIRTIO_NET_CTRL_MAC_ADDR_SET: u8 = 1;

const VIRTIO_NET_CTRL_VLAN: u64 = 2;
const VIRTIO_NET_CTRL_VLAN_ADD: u64 = 0;
const VIRTIO_NET_CTRL_VLAN_DEL: u64 = 1;

const VIRTIO_NET_CTRL_ANNOUNCE: u8 = 3;
const VIRTIO_NET_CTRL_ANNOUNCE_ACK: u8 = 0;

struct VirtioNetCtrlMq {
    virtqueue_pairs: u16,
}

const VIRTIO_NET_CTRL_MQ: u16 = 4;
const VIRTIO_NET_CTRL_MQ_VQ_PAIRS_SET: u16 = 0;
const VIRTIO_NET_CTRL_MQ_VQ_PAIRS_MIN: u16 = 1;
const VIRTIO_NET_CTRL_MQ_VQ_PAIRS_MAX: u16 = 0x8000;
