//! This is the configuration module it contains configuration entries for the kernel

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct KernelConfig {
    pub machine_name: String,
    pub acpi: bool,
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
    pub fn check_field_string(&self, field: &str, valcheck: &str) -> bool {
        let mut map = match serde_value::to_value(self) {
            Ok(serde_value::Value::Map(map)) => map,
            _ => panic!("expected a struct"),
        };

        let key = serde_value::Value::String(field.to_owned());
        let value = match map.remove(&key) {
            Some(value) => value,
            None => panic!("no such field"),
        };

        let check = match value {
            serde_value::Value::Bool(a) => a.to_string(),
            serde_value::Value::U8(_) => todo!(),
            serde_value::Value::U16(_) => todo!(),
            serde_value::Value::U32(_) => todo!(),
            serde_value::Value::U64(_) => todo!(),
            serde_value::Value::I8(_) => todo!(),
            serde_value::Value::I16(_) => todo!(),
            serde_value::Value::I32(_) => todo!(),
            serde_value::Value::I64(_) => todo!(),
            serde_value::Value::F32(_) => todo!(),
            serde_value::Value::F64(_) => todo!(),
            serde_value::Value::Char(_) => todo!(),
            serde_value::Value::String(_) => todo!(),
            serde_value::Value::Unit => todo!(),
            serde_value::Value::Option(value) => todo!(),
            serde_value::Value::Newtype(value) => todo!(),
            serde_value::Value::Seq(values) => todo!(),
            serde_value::Value::Map(btree_map) => todo!(),
            serde_value::Value::Bytes(items) => todo!(),
        };
        println!("Need to compare {:?} and {:?}", check, valcheck);
        check == valcheck
    }

    /// Returns the value of a field by name
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
