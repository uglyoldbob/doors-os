//! Code for running the virtualbox emulator

/// The virtualbox emulator
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct VirtualBox {}

impl super::EmulationTrait for VirtualBox {
    fn build_config(&self, _disk: &crate::Disk) {}

    fn run(
        &self,
        _local: &super::LocalConfiguration,
    ) -> Result<Option<std::process::Child>, std::io::Error> {
        todo!();
    }

    fn simple_name(&self) -> &str {
        "virtualbox"
    }
}
