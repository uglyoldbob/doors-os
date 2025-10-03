use super::OperatingSystemDetectorTrait;
use crate::DumpFile;

use object::{Object, ObjectSymbol};

#[derive(Clone, Debug, Default)]
pub struct DoorsOs {}

impl super::OperatingSystemTrait for DoorsOs {
    fn activity(&self, action: &str) {
        match action {
            "version" => {}
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
