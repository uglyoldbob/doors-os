use super::OperatingSystemDetectorTrait;
use crate::{DumpFile, symbols::SymbolsTrait};

#[derive(Clone, Debug, Default)]
pub struct DoorsOs {}

impl super::OperatingSystemTrait for DoorsOs {
    fn activity(&self, data: &DumpFile, kernel: &crate::symbols::Symbols<'_>, action: &str) {
        match action {
            "page_tables" => {
                if let Some(s) = kernel.find_symbol("PAGE_TABLE_PML4_BOOT", |a| {
                    format!("{:#}", rustc_demangle::demangle(a))
                }) {
                    println!("Found symbol for the page table {:x}", s.address());
                }
            }
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
                if let Some(s) = kernel.find_symbol("kernel::scheduler::SCHEDULER", |a| {
                    format!("{:#}", rustc_demangle::demangle(a))
                }) {
                    println!("Found symbol for the scheduler {:x}", s.address());
                    println!(
                        "Data is {:02x?}",
                        data.get_slice_with_length(s.address(), 128)
                    );
                    let addr = s.address() + 8;
                    let d = data
                        .get_slice_with_length(addr, std::mem::size_of::<usize>())
                        .unwrap();
                    let mut d2 = [0u8; std::mem::size_of::<usize>()];
                    d2.copy_from_slice(&d);
                    let da = usize::from_le_bytes(d2);
                    println!("Got address {:x}", da);
                }
            }
            "test" => println!("This is a test action"),
            _ => println!("Unknown action {}", action),
        }
    }
}

impl OperatingSystemDetectorTrait for DoorsOs {
    fn detect(&self, data: &DumpFile, kernel: &crate::symbols::Symbols<'_>) -> bool {
        let banner_search = data.find_subslice("DoorsOsIdentifier".as_bytes()).unwrap();

        let banner_symbol = kernel
            .find_symbol("KERNEL_STRING", |a| a.to_string())
            .unwrap();
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
