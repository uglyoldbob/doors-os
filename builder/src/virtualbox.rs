//! Code for running the virtualbox emulator

/// The virtualbox emulator
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct VirtualBox {}

impl VirtualBox {
    /// Get the port and irq for serial ports used
    fn get_serial_port_details(port: u8) -> (u16, u8) {
        match port {
            0 => (0x3f8, 4),
            1 => (0x2f8, 3),
            2 => (0x3e8, 4),
            3 => (0x2e8, 3),
            _ => unimplemented!(),
        }
    }
}

impl super::EmulationTrait for VirtualBox {
    fn build_config(
        &self,
        disk: &crate::Disk,
        common: &super::EmulatorConfig,
        local: &super::LocalConfiguration,
        s: std::path::PathBuf,
    ) {
        let _ = std::fs::remove_file("./doors-os-64/doors-os-64.vbox");
        std::process::Command::new(local.vboxmanage_path())
            .args([
                "createvm",
                "--name",
                "doors-os-64",
                "--ostype",
                "\"Doors\"",
                "--register",
                "--basefolder",
                std::env::current_dir().unwrap().to_str().unwrap(),
            ])
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        for (i, serial_id) in common.serial_ports.iter().enumerate() {
            let i = i + 1;
            let port = &local.serial_ports[*serial_id];
            let (io, irq) = Self::get_serial_port_details(i as u8);
            println!("SERIAL PORT DETAILS {:x} {} {:?}", io, irq, port);
            match port {
                super::SerialConfig::File(f) => {
                    std::process::Command::new(local.vboxmanage_path())
                        .args([
                            "modifyvm",
                            "doors-os-64",
                            &format!("--uart{}", i),
                            &format!("{}", io),
                            &format!("{}", irq),
                            &format!("--uartmode{}", i),
                            "file",
                            f.to_str().unwrap(),
                        ])
                        .spawn()
                        .unwrap()
                        .wait()
                        .unwrap();
                }
                super::SerialConfig::TcpServer(port) => {
                    std::process::Command::new(local.vboxmanage_path())
                        .args([
                            "modifyvm",
                            "doors-os-64",
                            &format!("--uart{}", i),
                            &format!("{}", io),
                            &format!("{}", irq),
                            &format!("--uartmode{}", i),
                            "tcpserver",
                            &format!("{}", port),
                        ])
                        .spawn()
                        .unwrap()
                        .wait()
                        .unwrap();
                }
                super::SerialConfig::TcpClient(port) => {
                    std::process::Command::new(local.vboxmanage_path())
                        .args([
                            "modifyvm",
                            "doors-os-64",
                            &format!("--uart{}", i),
                            &format!("{}", io),
                            &format!("{}", irq),
                            &format!("--uartmode{}", i),
                            "tcpclient",
                            &format!("{}", port),
                        ])
                        .spawn()
                        .unwrap()
                        .wait()
                        .unwrap();
                }
                super::SerialConfig::Real(p) => {
                    std::process::Command::new(local.vboxmanage_path())
                        .args([
                            "modifyvm",
                            "doors-os-64",
                            &format!("--uart{}", i),
                            &format!("{}", io),
                            &format!("{}", irq),
                            &format!("--uartmode{}", i),
                            p,
                        ])
                        .spawn()
                        .unwrap()
                        .wait()
                        .unwrap();
                }
                super::SerialConfig::Nothing => {}
            }
        }

        for (i, nid) in common.net_devs.iter().enumerate() {
            let net_name = local.net_devs[*nid].virtualbox.as_ref().unwrap();
            let nicnum = i + 1;
            std::process::Command::new(local.vboxmanage_path())
                .args([
                    "modifyvm",
                    "doors-os-64",
                    &format!("--nic{}", nicnum),
                    "hostonly",
                    &format!("--hostonlyadapter{}", nicnum),
                    net_name,
                ])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
            std::process::Command::new(local.vboxmanage_path())
                .args([
                    "modifyvm",
                    "doors-os-64",
                    &format!("--macaddress{}", nicnum),
                    "525400123456",
                ])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
            std::process::Command::new(local.vboxmanage_path())
                .args([
                    "modifyvm",
                    "doors-os-64",
                    &format!("--nictype{}", nicnum),
                    "82540EM",
                ])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
        }

        match disk {
            super::Disk::Network(_p) => {}
            super::Disk::Cd(p) => {
                std::process::Command::new(local.vboxmanage_path())
                    .args([
                        "storagectl",
                        "doors-os-64",
                        "--name",
                        "\"IDE Controller\"",
                        "--add",
                        "ide",
                        "--controller",
                        "PIIX4",
                    ])
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();
                std::process::Command::new(local.vboxmanage_path())
                    .args([
                        "storageattach",
                        "doors-os-64",
                        "--storagectl",
                        "\"IDE Controller\"",
                        "--port",
                        "1",
                        "--device",
                        "0",
                        "--type",
                        "dvddrive",
                        "--medium",
                        p.to_str().unwrap(),
                    ])
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();
            }
        }

        use std::io::Write;
        let mut config = String::new();

        config.push_str(&format!("add-symbol-file {}\n", s.to_str().unwrap()));
        config.push_str("disp /i $pc\n");
        config.push_str("target remote :12345\n");

        let f = "./gdb_config.gdb";
        let mut configf = std::fs::File::create(f).expect("Failed to create gdb configuration");
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
        cmakelists.push_str("\tDEPENDS boot_disk\n");
        cmakelists.push_str(&format!(
            "\tCOMMAND {} --startvm doors-os-64\n",
            super::LocalConfiguration::escape_path(&local.virtualbox_path()),
        ));
        cmakelists.push_str(")\n");

        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tdebug\n");
        cmakelists.push_str("\tDEPENDS boot_disk disassemble\n");
        cmakelists.push_str(&format!(
            "\tCOMMAND {} --startvm doors-os-64 --dbg --debug\n",
            super::LocalConfiguration::escape_path(&local.virtualbox_path()),
        ));
        cmakelists.push_str(")\n");

        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tgdb\n");
        cmakelists.push_str("\tDEPENDS boot_disk disassemble\n");
        cmakelists.push_str(&format!(
            "\tCOMMAND {} -x gdb_config.gdb\n",
            super::LocalConfiguration::escape_path(&local.gdb_path())
        ));
        cmakelists.push_str(")\n");
    }

    fn simple_name(&self) -> &str {
        "virtualbox"
    }
}
