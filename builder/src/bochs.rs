//! Code for running the bochs emulator

use gimli::EndianReader;

/// The bochs emulator
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Bochs {}

impl super::EmulationTrait for Bochs {
    fn custom_debug_symbols(&self, s: std::path::PathBuf) {
        use std::io::{Read, Write};
        let mut f = std::fs::File::open(s).unwrap();
        let mut contents = Vec::new();
        f.read_to_end(&mut contents).unwrap();
        let o = object::File::parse(&*contents).unwrap();
        use object::Object;
        use object::ObjectSection;
        let d = gimli::Dwarf::load(|id| {
            let secname = id.name();
            let data = o
                .section_by_name_bytes(secname.as_bytes())
                .ok_or(())
                .map(|d| d.data());
            match data {
                Ok(Ok(d)) => Ok::<EndianReader<gimli::LittleEndian, &[u8]>, ()>(EndianReader::new(
                    d,
                    gimli::LittleEndian,
                )),
                _ => Ok::<EndianReader<gimli::LittleEndian, &[u8]>, ()>(EndianReader::new(
                    &[],
                    gimli::LittleEndian,
                )),
            }
        });
        let mut syms = String::new();
        if let Ok(d) = d {
            let mut dunit = d.units();
            while let Some(header) = dunit.next().unwrap() {
                // Parse the abbreviations and other information for this compilation unit.
                let unit = d.unit(header).unwrap();

                // Iterate over all of this compilation unit's entries.
                let mut entries = unit.entries();
                while let Some((_, entry)) = entries.next_dfs().unwrap() {
                    // If we find an entry for a function, print it.
                    if entry.tag() == gimli::DW_TAG_subprogram {
                        let mut attrs = entry.attrs();
                        let mut fdata = String::new();
                        let mut printme = false;
                        let mut pc_low = None;
                        while let Ok(Some(attr)) = attrs.next() {
                            fdata.push_str(&format!(
                                "Found a function with attr: {:x?}\n",
                                attr.name()
                            ));
                            if attr.name() == gimli::DW_AT_low_pc {
                                if let gimli::read::AttributeValue::Addr(a) = attr.value() {
                                    printme = true;
                                    fdata.push_str(&format!("pc low is {:x}\n", a));
                                    pc_low = Some(a)
                                }
                            }
                            if let Some(pc_low) = pc_low {
                                if attr.name() == gimli::DW_AT_linkage_name {
                                    let v = attr.value();
                                    if let gimli::read::AttributeValue::DebugStrRef(a) = v {
                                        let n2 = d.string(a).unwrap();
                                        let name = std::str::from_utf8(&n2).unwrap().to_string();
                                        fdata.push_str(&format!(
                                            "Linkage name ref is {} @ {:x}\n",
                                            name, pc_low
                                        ));
                                        if pc_low != 0 {
                                            syms.push_str(&format!("{:x} {}\n", pc_low, name));
                                        }
                                    }
                                }
                            }
                        }
                        if false {
                            if printme {
                                println!("Process function");
                                print!("{}", fdata);
                            }
                        }
                    }
                }
            }
        }
        let mut configf = std::fs::File::create("./symbols_bochs")
            .expect("Failed to create bochs debug symbols file");
        configf
            .write_all(syms.as_bytes())
            .expect("Failed to save bochs debug symbols file");
    }

    fn build_config(
        &self,
        disk: &super::Disk,
        common: &super::EmulatorConfig,
        local: &super::LocalConfiguration,
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
            let net_name = &local.net_devs[*nid];
            #[cfg(target_os = "linux")]
            {
                config.push_str(&format!(
                    "e1000: enabled=1, mac=52:54:00:12:34:56, ethmod=linux, ethdev={}\n",
                    net_name
                ));
            }
            #[cfg(target_os = "windows")]
            {
                config.push_str(&format!(
                    "e1000: enabled=1, mac=52:54:00:12:34:56, ethmod=win32, ethdev={}\n",
                    net_name
                ));
            }
        }

        config.push_str("com1: enabled=1, mode=file, dev=serial.log\n");
        config.push_str("com2: enabled=1, mode=file, dev=serial2.log\n");
        config.push_str("debug_symbols: file=./symbols_bochs\n");
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
        common: &super::EmulatorConfig,
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

    fn simple_name(&self) -> &str {
        "bochs"
    }
}
