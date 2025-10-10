//! Contains the definition for a loopback network device

doors_macros2::enum_export_builder! {
    doors_macros2::enum_export!(NetworkAdapter, NetworkLoopback);
}

#[derive(Clone, Default)]
#[doors_macros::enum_variant(NetworkAdapter)]
/// Loopback network driver
pub struct NetworkLoopback {}

impl super::NetworkAdapterTrait for NetworkLoopback {
    async fn get_mac_address(&mut self) -> super::MacAddress {
        let a = [1, 2, 3, 4, 5, 6];
        super::MacAddress::from(&a[..])
    }

    async fn send_packet(&mut self, _packet: &[u8]) -> Result<(), ()> {
        todo!()
    }

    fn get_receiver(&self) -> &crate::OneWayStreamReader<super::RawEthernetPacket> {
        todo!()
    }
}
