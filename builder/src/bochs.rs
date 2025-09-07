//! Code for running the bochs emulator

/// The bochs emulator
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Bochs {}

impl super::EmulationTrait for Bochs {
    fn custom_debug_symbols(&self, cmakelists: &mut String, s: std::path::PathBuf) {
        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tdebug_symbols_kernel\n");
        cmakelists.push_str("\tBYPRODUCTS target\n");
        cmakelists.push_str(&format!("\tCOMMAND cargo +nightly run --bin bochs-debug -- --binary={} --output=./symbols_bochs\n", s.display()));
        cmakelists.push_str(")\n");
    }

    fn build_config(
        &self,
        disk: &super::Disk,
        common: &super::EmulatorConfig,
        local: &super::LocalConfiguration,
        _s: std::path::PathBuf,
    ) {
        use std::io::Write;
        let mut config: String = String::new();

        #[cfg(target_os = "linux")]
        {
            config.push_str("romimage: file=\"/usr/share/seabios/bios-256k.bin\"\n");
            config
                .push_str("vgaromimage: file =\"/usr/share/seabios/vgabios-bochs-display.bin\"\n");
        }

        for nid in &common.net_devs {
            let net_name = local.net_devs[*nid].bochs.as_ref().unwrap();
            #[cfg(target_os = "linux")]
            {
                config.push_str(&format!(
                    "e1000: enabled=1, mac=52:54:00:12:34:56, ethmod=linux, ethdev={}\n",
                    net_name
                ));
            }
            #[cfg(target_os = "windows")]
            {
                config.push_str("e1000: enabled=1, mac=52:54:00:12:34:56, ethmod=slirp\n");
            }
        }

        for (i, serial_id) in common.serial_ports.iter().enumerate() {
            let i = i + 1;
            let port = &local.serial_ports[*serial_id];
            match port {
                super::SerialConfig::File(f) => {
                    config.push_str(&format!(
                        "com{}: enabled=1, mode=file, dev={}\n",
                        i,
                        f.to_str().unwrap()
                    ));
                }
                super::SerialConfig::TcpServer(port) => {
                    #[cfg(target_os = "windows")]
                    {
                        config.push_str(&format!(
                            "com{}: enabled=1, mode=socket-server, dev=localhost:{}\n",
                            i, port
                        ));
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        panic!("Tcp server serial port {} not supported on bochs", port);
                    }
                }
                super::SerialConfig::TcpClient(port) => {
                    #[cfg(target_os = "windows")]
                    {
                        config.push_str(&format!(
                            "com{}: enabled=1, mode=socket-client, dev=localhost:{}\n",
                            i, port
                        ));
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        panic!("Tcp server serial port {} not supported on bochs", port);
                    }
                }
                super::SerialConfig::Real(p) => {
                    config.push_str(&format!("com{}: enabled=1, mode=raw, dev={}\n", i, p));
                }
                super::SerialConfig::Nothing => {}
            }
        }

        config.push_str("debug_symbols: file=./symbols_bochs\n");
        config.push_str("magic_break: enabled=1\n");
        config.push_str("#debug: action=ignore, e1000a=report\n");
        match disk {
            crate::Disk::Cd(p) => {
                config.push_str(&format!(
                    "ata0-slave: type=cdrom, path={}, status=inserted\n",
                    p.display()
                ));
                config.push_str("boot: cdrom\n");
            }
            crate::Disk::Network(_p) => {}
        }
        let f = "./bochs_config.txt";
        let mut configf = std::fs::File::create(f).expect("Failed to create bochs configuration");
        configf
            .write_all(config.as_bytes())
            .expect("Failed to save configuration file");
    }

    fn run(
        &self,
        cmakelists: &mut String,
        _common: &super::EmulatorConfig,
        local: &super::LocalConfiguration,
        _s: std::path::PathBuf,
    ) {
        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\trun\n");
        cmakelists.push_str("\tDEPENDS boot_disk disassemble debug_symbols_kernel\n");
        cmakelists.push_str(&format!(
            "\tCOMMAND {} -f bochs_config.txt -q\n",
            super::LocalConfiguration::escape_path(&local.bochs_path())
        ));
        cmakelists.push_str(")\n");
    }

    fn simple_name(&self) -> &str {
        "bochs"
    }
}
