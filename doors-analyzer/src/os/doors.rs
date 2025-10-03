use super::OperatingSystemDetectorTrait;
use crate::DumpFile;

use object::{Object, ObjectSymbol};

#[derive(Clone, Debug, Default)]
pub struct DoorsOs {}

impl super::OperatingSystemTrait for DoorsOs {
    fn activity(&self, data: &DumpFile, kernel: &object::File<'_>, action: &str) {
        match action {
            "version" => {
                let r = regex::bytes::Regex::new(r"doors version (\d+.\d+.\d+)").unwrap();
                for a in data.find_with_regex(r) {
                    println!(
                        "Found version {}",
                        str::from_utf8(a.get(1).unwrap().as_bytes()).unwrap()
                    );
                }
            }
            "scheduler" => {
                for s in kernel.symbols() {
                    println!("SYMBOL {:?}", s.name());
                }
                let banner_symbol = kernel
                    .symbol_by_name("_ZN6kernel9scheduler9SCHEDULER17h5971e05ddd8d9caaE")
                    .unwrap();
                println!(
                    "Scheduler address2 is 0x{:x} {:02x?}",
                    banner_symbol.address()+8,
                    data.get_slice_with_length(8+banner_symbol.address() as usize, 64)
                );
                let banner_address = data
                    .get_slice_with_length(8+banner_symbol.address() as usize, 8)
                    .unwrap();
                let mut ba_buf = [0; 8];
                ba_buf.copy_from_slice(&banner_address[..]);
                println!("DEBUG: {:02x?}", ba_buf);
                let address = usize::from_le_bytes(ba_buf);
                println!(
                    "Scheduler address is 0x{:x} {:02x?}",
                    address,
                    data.get_slice_with_length(address, 64)
                );
                let inner_scheduler = data
                .get_slice_with_length(address, 8)
                .unwrap();
                let mut ba_buf = [0; 8];
                ba_buf.copy_from_slice(&inner_scheduler[..]);
                let inner_address = usize::from_le_bytes(ba_buf);
                println!("Inner Scheduler address is 0x{:x}", inner_address);
            }
            "test" => println!("This is a test action"),
            _ => println!("Unknown action {}", action),
        }
    }
}

impl OperatingSystemDetectorTrait for DoorsOs {
    fn detect(&self, data: &DumpFile, kernel: &object::File<'_>) -> bool {
        let banner_search = data.find_subslice("DoorsOsIdentifier".as_bytes()).unwrap();

        let banner_symbol = kernel.symbol_by_name("KERNEL_STRING").unwrap();
        let banner_address = data
            .get_slice(banner_symbol.address() as usize..banner_symbol.address() as usize + 8)
            .unwrap();
        let mut ba_buf = [0; 8];
        ba_buf.copy_from_slice(&banner_address[..]);
        let banner_address = usize::from_le_bytes(ba_buf);

        banner_address == banner_search
    }

    fn get_os(&self) -> Option<super::OperatingSystem> {
        Some(super::OperatingSystem::Doors(self.clone()))
    }
}
