use crate::error::NetError;

/// Low level trait shared by net devices. Handles direct data transmission
pub trait NetDevice {
    fn send_packet(&mut self, payload: &[u8]) -> Result<(), NetError>;
    fn poll_receive(&mut self, rx_callback: impl FnMut(&[u8]));
    fn mac_address(&self) -> [u8; 6];
}
