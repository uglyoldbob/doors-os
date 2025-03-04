//! Code for running the qemu emulator

/// The qemu emulator
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Qemu {}

impl super::EmulationTrait for Qemu {
    fn build_config(&self, _disk: &crate::Disk, local: &super::LocalConfiguration) {}

    fn run(
        &self,
        _local: &super::LocalConfiguration,
    ) -> Result<Option<std::process::Child>, std::io::Error> {
        todo!();
    }

    fn simple_name(&self) -> &str {
        "qemu"
    }
}
