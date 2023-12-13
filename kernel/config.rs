//! This is the configuration module it contains configuration entries for the kernel

#[derive(serde::Deserialize)]
pub struct KernelConfig {
    pub machine_name: String,
}

impl KernelConfig {
    /// Returns the architecture that is supposed to be used for the machine
    pub fn get_arch(&self) -> String {
        match self.machine_name.as_str() {
            "stm32f769i-disco" => "arm",
            "pc64" => "x86_64",
            _ => "unknown",
        }
        .to_string()
    }
}
