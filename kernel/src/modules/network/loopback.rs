//! Contains the definition for a loopback network device

#[derive(Clone, Default)]
/// Loopback network driver
pub struct NetworkLoopback {}

impl super::NetworkAdapterTrait for NetworkLoopback {
    async fn get_mac_address(&mut self) -> super::MacAddress {
        let a = [1, 2, 3, 4, 5, 6];
        super::MacAddress::from(&a[..])
    }

    fn get_receiver(&self) -> Option<crate::OneWayStreamReader<super::RawEthernetPacket>> {
        todo!()
    }

    fn get_sender(&self) -> crate::OneWayStreamWriter<super::RawEthernetPacket> {
        todo!()
    }

    async fn send_pending_packets(&mut self) {
        todo!()
    }
}
