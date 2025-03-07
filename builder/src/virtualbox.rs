//! Code for running the virtualbox emulator

/// The virtualbox emulator
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct VirtualBox {}

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

        std::process::Command::new(local.vboxmanage_path())
            .args([
                "modifyvm",
                "doors-os-64",
                "--uart1",
                "0x3f8",
                "4",
                "--uartmode1",
                "file",
                "serial.log",
            ])
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        std::process::Command::new(local.vboxmanage_path())
            .args([
                "modifyvm",
                "doors-os-64",
                "--uart2",
                "0x2f8",
                "3",
                "--uartmode2",
                "tcpserver",
                "1234",
            ])
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        for (i, nid) in common.net_devs.iter().enumerate() {
            let net_name = &local.net_devs[*nid];
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
    }

    fn run(
        &self,
        cmakelists: &mut String,
        _common: &super::EmulatorConfig,
        local: &super::LocalConfiguration,
        s: std::path::PathBuf,
    ) {
        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\trun\n");
        cmakelists.push_str(&format!(
            "\tCOMMAND {} --startvm doors-os-64\n",
            super::LocalConfiguration::escape_path(&local.virtualbox_path()),
        ));
        cmakelists.push_str(")\n");

        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tdebug\n");
        cmakelists.push_str(&format!(
            "\tCOMMAND {} --startvm doors-os-64 --dbg --debug\n",
            super::LocalConfiguration::escape_path(&local.virtualbox_path()),
        ));
        cmakelists.push_str(")\n");
    }

    fn simple_name(&self) -> &str {
        "virtualbox"
    }
}
