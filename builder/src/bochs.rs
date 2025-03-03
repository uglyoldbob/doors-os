//! Code for running the bochs emulator

/// The bochs emulator
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Bochs {}

impl super::EmulationTrait for Bochs {
    fn build_config(&self, disk: &super::Disk) {
        use std::io::Write;
        let mut config: String = String::new();

        config.push_str("romimage: file=\"/usr/share/seabios/bios-256k.bin\"\n");
        config.push_str("vgaromimage: file =\"/usr/share/seabios/vgabios-bochs-display.bin\"\n");
        config.push_str("e1000: enabled=1, mac=52:54:00:12:34:56, ethmod=linux, ethdev=vboxnet0\n");
        config.push_str("com1: enabled=1, mode=file, dev=serial.log\n");
        config.push_str("com2: enabled=1, mode=file, dev=serial2.log\n");
        config.push_str("debug_symbols: file=./kernel/symbols_bochs\n");
        config.push_str("magic_break: enabled=1\n");
        config.push_str("#debug: pic=report\n");
        match disk {
            crate::Disk::Cd(p) => {
                config.push_str(&format!(
                    "ata0-slave: type=cdrom, path={}, status=inserted\n",
                    p.display()
                ));
                config.push_str("boot: cdrom\n");
            }
        }
        let f = "./bochs_config.txt";
        let mut configf = std::fs::File::create(f).expect("Failed to create bochs configuration");
        configf
            .write_all(config.as_bytes())
            .expect("Failed to save configuration file");
    }

    fn run(
        &self,
        local: &super::LocalConfiguration,
    ) -> Result<Option<std::process::Child>, std::io::Error> {
        let mut b = if let Some(p) = &local.bochs_path {
            std::process::Command::new(p)
        } else {
            std::process::Command::new("bochs")
        };
        let b = b.args(["-f", "bochs_config.txt", "-q"]);
        b.spawn().map(Some)
    }
}
