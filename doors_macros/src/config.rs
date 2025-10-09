//! This is the configuration module it contains configuration entries for the kernel

use std::collections::HashSet;

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct KernelConfig {
    /// The convenience name of the machine
    pub machine_name: String,
    /// Should acpi support be compiled into the kernel?
    pub acpi: bool,
    /// Should gdbstub support be enabled?
    pub gdbstub: bool,
    /// Should memory management debugging be enabled?
    pub mm_debug: bool,
    /// Should the kernel test functionality be enabled?
    pub test: bool,
    /// Should network functionality be included?
    pub network: bool,
    /// What modules should be compiled into the kernel
    pub modules: HashSet<String>,
}

impl KernelConfig {
    /// Returns the architecture that is supposed to be used for the machine
    #[allow(dead_code)]
    pub fn get_arch(&self) -> String {
        match self.machine_name.as_str() {
            "stm32f769i-disco" => "arm",
            "pc64" => "x86_64",
            _ => "unknown",
        }
        .to_string()
    }

    /// Returns the string value of a field by name, as it would be in the serialized format
    pub fn check_field(&self, field: &str, valcheck: &str) -> bool {
        let toml = toml::to_string(self).expect("Fail 1");
        let toml = toml.parse::<toml::Table>().expect("Fail 2");
        let tomlcheck = format!("{} = '{}'", field, valcheck);
        let valcheck = tomlcheck.parse::<toml::Value>().expect("Fail 3");
        if !toml.contains_key(field) {
            panic!("Field not present in kernel config");
        }
        let config_val = format!("{} = '{}'", field, toml.get(field).unwrap());
        let config_val = config_val.parse::<toml::Value>().expect("Fail 4");
        valcheck == config_val
    }
}
