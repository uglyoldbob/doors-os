//! Code for running the qemu emulator

/// The qemu emulator
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Qemu {}

impl super::EmulationTrait for Qemu {
    fn build_config(&self, disk: &crate::Disk) {}

    fn run(&self) -> Result<Option<std::process::Child>, std::io::Error> {
        todo!();
    }
}
