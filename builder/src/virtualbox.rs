//! Code for running the virtualbox emulator

/// The virtualbox emulator
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct VirtualBox {}

impl super::EmulationTrait for VirtualBox {
    fn build_config(&self, disk: &crate::Disk) {}

    fn run(&self) -> Result<Option<std::process::Child>, std::io::Error> {
        todo!();
    }
}
