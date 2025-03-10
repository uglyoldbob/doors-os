//! Code for running the qemu emulator

/// The qemu emulator
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Qemu {}

impl Qemu {
    fn get_common_run(
        &self,
        common: &super::EmulatorConfig,
        local: &super::LocalConfiguration,
    ) -> String {
        let mut qemu = String::new();
        qemu.push_str(&format!(
            "{} ",
            super::LocalConfiguration::escape_path(&local.qemu_path())
        ));
        qemu.push_str("-cdrom cd64.iso -m 8 ");
        for serial_id in common.serial_ports.iter() {
            let port = &local.serial_ports[*serial_id];
            match port {
                super::SerialConfig::File(f) => {
                    qemu.push_str(&format!("-serial file:{} ", f.to_str().unwrap()));
                }
                super::SerialConfig::TcpServer(port) => {
                    qemu.push_str(&format!("-serial tcp::{},server,nowait,nodelay ", port));
                }
                super::SerialConfig::TcpClient(port) => {
                    qemu.push_str(&format!("-serial tcp::{},nodelay ", port));
                }
                super::SerialConfig::Real(p) => {
                    qemu.push_str(&format!("-serial {} ", p));
                }
                super::SerialConfig::Nothing => {}
            }
        }
        qemu.push_str("-netdev user,id=u1 -device e1000,netdev=u1");
        qemu
    }
}

impl super::EmulationTrait for Qemu {
    fn build_config(
        &self,
        _disk: &crate::Disk,
        common: &super::EmulatorConfig,
        local: &super::LocalConfiguration,
        s: std::path::PathBuf,
    ) {
        use std::io::Write;
        let mut config = String::new();

        let mut qemu = String::new();
        qemu.push_str(&self.get_common_run(common, local));
        qemu.push_str(" -gdb stdio");

        config.push_str(&format!("add-symbol-file {}\n", s.to_str().unwrap()));
        config.push_str("define exit\n\tmonitor quit\n\tquit\nend\n");
        config.push_str("disp /i $pc\n");
        config.push_str("target remote :1234\n");

        let f = "./gdb_config.gdb";
        let mut configf =
            std::fs::File::create(f).expect("Failed to create gdb stub configuration");
        configf
            .write_all(config.as_bytes())
            .expect("Failed to save configuration file");

        let mut config = String::new();

        config.push_str(&format!("add-symbol-file {}\n", s.to_str().unwrap()));
        config.push_str("disp /i $pc\n");
        config.push_str("target remote :12345 \n");

        let f = "./gdb_stub.gdb";
        let mut configf = std::fs::File::create(f).expect("Failed to create gdb configuration");
        configf
            .write_all(config.as_bytes())
            .expect("Failed to save configuration file");
    }

    fn run(
        &self,
        cmakelists: &mut String,
        common: &super::EmulatorConfig,
        local: &super::LocalConfiguration,
        _s: std::path::PathBuf,
    ) {
        let mut qemu = String::new();
        qemu.push_str("\tCOMMAND ");
        qemu.push_str(&self.get_common_run(common, local));
        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\trun\n");
        cmakelists.push_str("\tDEPENDS boot_disk disassemble\n");
        cmakelists.push_str(&qemu);
        cmakelists.push_str("\n)\n");

        let mut qemu = String::new();
        qemu.push_str("\tCOMMAND ");
        qemu.push_str(&self.get_common_run(common, local));
        qemu.push_str(" -s -S");
        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tdebug\n");
        cmakelists.push_str("\tDEPENDS boot_disk disassemble\n");
        cmakelists.push_str(&qemu);
        cmakelists.push_str("\n)\n");

        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tdebug2\n");
        cmakelists.push_str("\tDEPENDS boot_disk disassemble\n");
        cmakelists.push_str(&format!(
            "\tCOMMAND {} -x gdb_config.gdb\n",
            super::LocalConfiguration::escape_path(&local.gdb_path())
        ));
        cmakelists.push_str(")\n");

        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tgdb\n");
        cmakelists.push_str("\tDEPENDS boot_disk disassemble\n");
        cmakelists.push_str(&format!(
            "\tCOMMAND {} -x gdb_stub.gdb\n",
            super::LocalConfiguration::escape_path(&local.gdb_path())
        ));
        cmakelists.push_str(")\n");
    }

    fn simple_name(&self) -> &str {
        "qemu"
    }
}
