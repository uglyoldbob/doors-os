//! This is the configuration module it contains configuration entries for the kernel

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct KernelConfig {
    pub machine_name: String,
    pub acpi: bool,
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

    pub fn get_field<R>(&self, field: &str) -> R
    where
        R: serde::de::DeserializeOwned,
    {
        let mut map = match serde_value::to_value(self) {
            Ok(serde_value::Value::Map(map)) => map,
            _ => panic!("expected a struct"),
        };

        let key = serde_value::Value::String(field.to_owned());
        let value = match map.remove(&key) {
            Some(value) => value,
            None => panic!("no such field"),
        };

        match R::deserialize(value) {
            Ok(r) => r,
            Err(_) => panic!("wrong type?"),
        }
    }
}
