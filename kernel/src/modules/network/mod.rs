//! Networking code for the kernel

use crate::modules::video::TextDisplayTrait;

use alloc::{borrow::ToOwned, collections::btree_map::BTreeMap, string::String, vec::Vec};

use crate::{Locked, LockedArc};

doors_macros::declare_enum!(NetworkAdapter);

pub mod intel;

doors_macros2::enum_reexport!(intel);

/// Represents all network adapters for the kernel
static NETWORK_ADAPTERS: Locked<BTreeMap<String, LockedArc<NetworkAdapter>>> =
    Locked::new(BTreeMap::new());

/// Register a network adapter
pub fn register_network_adapter(na: NetworkAdapter) {
    let mut nal = NETWORK_ADAPTERS.lock();
    //TODO implement an automatic naming scheme
    use alloc::string::ToString;
    let name = "net0".to_string();
    doors_macros2::kernel_print!("Registering a network adapter for {}\r\n", name);
    nal.insert(name, LockedArc::new(na));
}

/// Grab a network adapter by name
pub fn get_network_adapter(s: &str) -> Option<LockedArc<NetworkAdapter>> {
    let nal = NETWORK_ADAPTERS.lock();
    for s in nal.keys() {
        doors_macros2::kernel_print!("There is a network adapter {}\r\n", s);
    }
    if nal.contains_key(s) {
        Some(nal.get(s).unwrap().to_owned())
    } else {
        None
    }
}

/// A mac address for a network adapter
#[derive(Clone, Copy, Debug)]
pub struct MacAddress {
    /// The bytes of the mac address
    address: [u8; 6],
}

impl From<u64> for MacAddress {
    fn from(value: u64) -> Self {
        let a = value.to_le_bytes();
        Self {
            address: [a[0], a[1], a[2], a[3], a[4], a[5]],
        }
    }
}

impl Into<u64> for MacAddress {
    fn into(self) -> u64 {
        let a: [u8; 8] = [
            self.address[0],
            self.address[1],
            self.address[2],
            self.address[3],
            self.address[4],
            self.address[5],
            0,
            0,
        ];
        u64::from_le_bytes(a)
    }
}

/// Test the mac address conversion to and from u64
#[doors_macros::doors_test]
fn mac_address_conversion_test() -> Result<(), ()> {
    let mac = MacAddress {
        address: [1, 2, 3, 4, 5, 6],
    };
    let b: u64 = mac.clone().into();
    assert_eq!(b, 0x060504030201);
    let mac2: MacAddress = b.into();
    assert_eq!(mac.address, mac2.address);
    Ok(())
}

/// An Ipv4 address
#[derive(Clone, Copy)]
pub struct IpV4 {
    /// The 4 parts of the address
    address: [u8; 4],
    /// The subnet mask
    mask: [u8; 4],
}

impl alloc::fmt::Debug for IpV4 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&alloc::format!(
            "{}.{}.{}.{} {}.{}.{}.{}",
            self.address[0],
            self.address[1],
            self.address[2],
            self.address[3],
            self.mask[0],
            self.mask[1],
            self.mask[2],
            self.mask[3]
        ))
    }
}

/// An Ipv6 address
#[derive(Clone, Copy)]
pub struct IpV6 {
    /// The 4 parts of the address
    address: [u16; 8],
    /// The prefix length
    prefix: u8,
}

impl alloc::fmt::Debug for IpV6 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut zeros: [bool; 8] = [false; 8];
        for (i, a) in zeros.iter_mut().enumerate() {
            *a = self.address[i] == 0;
        }
        let mut num_consecutive_zeros = [0; 8];
        {
            for i in 0..8 {
                if zeros[i] {
                    let mut j = i;
                    loop {
                        if j >= 8 {
                            break;
                        }
                        if !zeros[j] {
                            break;
                        }
                        j += 1;
                    }
                    num_consecutive_zeros[i] = j - i;
                }
            }
        }
        let mut regular_print = || {
            f.write_str(&alloc::format!(
                "{}:{}:{}:{}:{}:{}:{}:{}/{}",
                self.address[0],
                self.address[1],
                self.address[2],
                self.address[3],
                self.address[4],
                self.address[5],
                self.address[6],
                self.address[7],
                self.prefix
            ))
        };
        let max = num_consecutive_zeros
            .iter()
            .enumerate()
            .max_by(|(_i1, e1), (_i2, e2)| e1.cmp(e2));
        if let Some((i, max)) = max {
            if *max > 0 {
                let mut pdata: Vec<Option<u16>> = Vec::with_capacity(8);
                let mut index = 0;
                loop {
                    let calc = index >= i && index < (i + max);
                    if calc {
                        pdata.push(None);
                        index += *max;
                    } else {
                        pdata.push(Some(self.address[index]));
                        index += 1;
                    }
                    if index >= 8 {
                        break;
                    }
                }
                let maxlen = pdata.len() - 1;
                for (i, d) in pdata.iter().enumerate() {
                    if i == 0 {
                        if let Some(d) = d {
                            f.write_str(&alloc::format!("{:x}", d))?;
                        } else {
                            f.write_str(":")?;
                        }
                    } else if i == maxlen {
                        if let Some(d) = d {
                            f.write_str(&alloc::format!(":{:x}", d))?;
                        } else {
                            f.write_str("::")?;
                        }
                    } else if let Some(d) = d {
                        f.write_str(&alloc::format!(":{:x}", d))?;
                    } else {
                        f.write_str(":")?;
                    }
                }
                f.write_str(&alloc::format!("/{}", self.prefix))
            } else {
                regular_print()
            }
        } else {
            regular_print()
        }
    }
}

/// Test the ipv6 Debug implementation
#[doors_macros::doors_test]
fn ipv6_network_test() -> Result<(), ()> {
    let ipv6 = IpV6 {
        address: [1, 2, 3, 4, 5, 6, 7, 8],
        prefix: 4,
    };
    let t1 = alloc::format!("{:?}", ipv6);
    assert_eq!(t1, "1:2:3:4:5:6:7:8/4");

    let zeros: &[(&[u8], &str)] = &[
        (&[0], "::2:3:4:5:6:7:8/4"),
        (&[1], "1::3:4:5:6:7:8/4"),
        (&[2], "1:2::4:5:6:7:8/4"),
        (&[3], "1:2:3::5:6:7:8/4"),
        (&[4], "1:2:3:4::6:7:8/4"),
        (&[5], "1:2:3:4:5::7:8/4"),
        (&[6], "1:2:3:4:5:6::8/4"),
        (&[7], "1:2:3:4:5:6:7::/4"),
        (&[0, 1], "::3:4:5:6:7:8/4"),
        (&[0, 1, 2], "::4:5:6:7:8/4"),
        (&[0, 1, 2, 3], "::5:6:7:8/4"),
        (&[0, 1, 2, 3, 4], "::6:7:8/4"),
        (&[0, 1, 2, 3, 4, 5], "::7:8/4"),
        (&[0, 1, 2, 3, 4, 5, 6], "::8/4"),
        (&[1, 2], "1::4:5:6:7:8/4"),
        (&[1, 2, 3], "1::5:6:7:8/4"),
        (&[1, 2, 3, 4], "1::6:7:8/4"),
        (&[1, 2, 3, 4, 5], "1::7:8/4"),
        (&[1, 2, 3, 4, 5, 6], "1::8/4"),
        (&[1, 2, 3, 4, 5, 6, 7], "1::/4"),
        (&[2, 3], "1:2::5:6:7:8/4"),
        (&[2, 3, 4], "1:2::6:7:8/4"),
        (&[2, 3, 4, 5], "1:2::7:8/4"),
        (&[2, 3, 4, 7], "1:2::6:7:0/4"),
    ];
    for (zeros, check) in zeros.iter() {
        let mut ipc = ipv6;
        for z in zeros.iter() {
            ipc.address[*z as usize] = 0;
        }
        let t1 = alloc::format!("{:?}", ipc);
        assert_eq!(&t1, check);
    }
    Ok(())
}

/// A network adapter ip address
#[derive(Clone, Copy)]
pub enum IpAddress {
    /// An ipv4 address
    IpV4(IpV4),
    /// An ipv6 address
    IpV6(IpV6),
}

/// The trait that defines common functionality for network adapters
#[enum_dispatch::enum_dispatch]
pub trait NetworkAdapterTrait {
    /// Retrieve the mac address for the network adapter
    fn get_mac_address(&mut self) -> MacAddress;
}

/// A network adapter
#[doors_macros::fill_enum_with_variants(NetworkAdapterTrait)]
pub enum NetworkAdapter {}
