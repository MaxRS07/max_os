// VirtIO NET bitflags

/// Device handles packets with partial checksum. This “checksum offload” is a common feature on modern network cards.
pub const VIRTIO_NET_F_CSUM: u64 = 1;
/// Driver handles packets with partial checksum.
pub const VIRTIO_NET_F_GUEST_CSUM: u64 = 1 << 1;
/// Control channel offloads reconfiguration support.
pub const VIRTIO_NET_F_CTRL_GUEST_OFFLOADS: u64 = 1 << 2;
/// Device maximum MTU reporting is supported. If offered by the device, device advises driver about the value of its maximum MTU. If negotiated, the driver uses mtu as the maximum MTU value.
pub const VIRTIO_NET_F_MTU: u64 = 1 << (3);
/// Device has given MAC address.
pub const VIRTIO_NET_F_MAC: u64 = 1 << (5);
/// Driver can receive TSOv4.
pub const VIRTIO_NET_F_GUEST_TSO4: u64 = 1 << (7);
/// Driver can receive TSOv6.
pub const VIRTIO_NET_F_GUEST_TSO6: u64 = 1 << (8);
/// Driver can receive TSO with ECN.
pub const VIRTIO_NET_F_GUEST_ECN: u64 = 1 << (9);
/// Driver can receive UFO.
pub const VIRTIO_NET_F_GUEST_UFO: u64 = 1 << (10);
/// Device can receive TSOv4.
pub const VIRTIO_NET_F_HOST_TSO4: u64 = 1 << (11);
/// Device can receive TSOv6.
pub const VIRTIO_NET_F_HOST_TSO6: u64 = 1 << (12);
/// Device can receive TSO with ECN.
pub const VIRTIO_NET_F_HOST_ECN: u64 = 1 << (13);
/// Device can receive UFO.
pub const VIRTIO_NET_F_HOST_UFO: u64 = 1 << (14);
/// Driver can merge receive buffers.
pub const VIRTIO_NET_F_MRG_RXBUF: u64 = 1 << (15);
/// Configuration status field is available.
pub const VIRTIO_NET_F_STATUS: u64 = 1 << (16);
/// Control channel is available.
pub const VIRTIO_NET_F_CTRL_VQ: u64 = 1 << (17);
/// Control channel RX mode support.
pub const VIRTIO_NET_F_CTRL_RX: u64 = 1 << (18);
/// Control channel VLAN filtering.
pub const VIRTIO_NET_F_CTRL_VLAN: u64 = 1 << (19);
/// Driver can send gratuitous packets.
pub const VIRTIO_NET_F_GUEST_ANNOUNCE: u64 = 1 << (21);
/// Device supports multiqueue with automatic receive steering.
pub const VIRTIO_NET_F_MQ: u64 = 1 << (22);
/// Set MAC address through control channel.
pub const VIRTIO_NET_F_CTRL_MAC_ADDR: u64 = 1 << (23);

pub const VIRTIO_RING_F_EVENT_IDX: u64 = 1 << 29;
/// Device can process duplicated ACKs and report number of coalesced segments and duplicated ACKs
pub const VIRTIO_NET_F_RSC_EXT: u64 = 1 << (61);
/// Device may act as a standby for a primary device with the same MAC address.
pub const VIRTIO_NET_F_STANDBY: u64 = 1 << (62);

pub const VIRTIO_NET_HDR_F_NEEDS_CSUM: u64 = 1;
pub const VIRTIO_NET_HDR_F_DATA_VALID: u64 = 2;
pub const VIRTIO_NET_HDR_F_RSC_INFO: u64 = 4;
pub const VIRTIO_NET_HDR_GSO_NONE: u64 = 0;
pub const VIRTIO_NET_HDR_GSO_TCPV4: u64 = 1;
pub const VIRTIO_NET_HDR_GSO_UDP: u64 = 3;
pub const VIRTIO_NET_HDR_GSO_TCPV6: u64 = 4;
pub const VIRTIO_NET_HDR_GSO_ECN: u64 = 0x80;

/// Starting flags for my driver, adapt more features later
pub const BASIC_FLAGS: u64 =
    VIRTIO_NET_F_MAC | VIRTIO_NET_F_STATUS | VIRTIO_RING_F_EVENT_IDX | 1 << 32; // bit 32 marks version 1

pub struct VirtioNetHdr {
    pub flags: u8,
    pub gso_type: u8,
    pub hdr_len: u16,
    pub gso_size: u16,
    pub csum_start: u16,
    pub csum_offset: u16,
    pub num_buffers: u16,
}
impl VirtioNetHdr {
    pub fn empty() -> Self {
        Self {
            flags: 0,
            gso_type: 0,
            hdr_len: 0,
            gso_size: 0,
            csum_start: 0,
            csum_offset: 0,
            num_buffers: 0,
        }
    }
}

const RX_PAYLOAD_SIZE: usize = 1526;

pub struct RxBuffer {
    pub hdr: VirtioNetHdr,
    pub payload: [u8; RX_PAYLOAD_SIZE - size_of::<VirtioNetHdr>()],
}

impl RxBuffer {
    pub fn empty() -> Self {
        Self {
            hdr: VirtioNetHdr::empty(),
            payload: [0; Self::payload_size()],
        }
    }
    pub const fn payload_size() -> usize {
        RX_PAYLOAD_SIZE - size_of::<VirtioNetHdr>()
    }
}
