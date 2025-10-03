use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::DumpFile;

#[enum_dispatch::enum_dispatch]
pub trait OperatingSystemTrait {
    fn activity(&self, action: &str);
}

#[derive(Debug)]
#[enum_dispatch::enum_dispatch(OperatingSystemTrait)]
pub enum OperatingSystem {
    Doors(DoorsOs),
}

#[enum_dispatch::enum_dispatch]
pub trait OperatingSystemDetectorTrait {
    /// Detects the operating system, returns true if it was properly detected
    fn detect(&self, data: &DumpFile, kernel: &object::File<'_>) -> bool;
    /// Get the operating system object
    fn get_os(&self) -> Option<OperatingSystem>;
}

mod doors;
use doors::*;

#[derive(EnumIter)]
#[enum_dispatch::enum_dispatch(OperatingSystemDetectorTrait)]
pub enum OperatingSystemDetector {
    Doors(DoorsOs),
    None(u32),
}

impl OperatingSystemDetectorTrait for u32 {
    fn detect(&self, _data: &DumpFile, _kernel: &object::File<'_>) -> bool {
        true
    }

    fn get_os(&self) -> Option<OperatingSystem> {
        None
    }
}

impl OperatingSystemDetector {
    pub fn detect_os(data: &DumpFile, kernel: &object::File<'_>) -> Option<OperatingSystem> {
        for ost in Self::iter() {
            if ost.detect(data, kernel) {
                return ost.get_os();
            }
        }
        return None;
    }
}
