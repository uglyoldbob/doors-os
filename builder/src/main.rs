//! This utility is used to build a complete operating system

#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]

use clap::Parser;

mod bochs;
use bochs::*;

mod qemu;
use qemu::*;

mod virtualbox;
use virtualbox::*;

doors_macros::define_config!();

/// The trait that specifies how to build an emulator configuration and run it
#[enum_dispatch::enum_dispatch]
trait EmulationTrait {
    /// Build the config for the emulator
    fn build_config(&self, disk: &Disk);
    /// Run the emulator
    fn run(&self) -> Result<Option<std::process::Child>, std::io::Error>;
}

/// An emulation target that does nothing
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct NoEmulator {}

impl EmulationTrait for NoEmulator {
    fn build_config(&self, _disk: &Disk) {}

    fn run(&self) -> Result<Option<std::process::Child>, std::io::Error> {
        Ok(None)
    }
}

/// Specifies how the operating system is to be emulated or run
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[enum_dispatch::enum_dispatch(EmulationTrait)]
enum Emulation {
    /// The bochs emulator
    Bochs(Bochs),
    /// Qemu emulator
    Qemu(Qemu),
    /// Virtualbox emulator
    VirtualBox(VirtualBox),
    /// No emulator, do nothing (successfully)
    None(NoEmulator),
}

impl Default for Emulation {
    fn default() -> Self {
        Self::None(NoEmulator {})
    }
}

/// Command line arguments for the tool
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the configuration to use
    #[arg(short, long)]
    name: Option<std::path::PathBuf>,
    /// Should the final configuration be saved to a file?
    #[arg(long)]
    save: Option<std::path::PathBuf>,
}

/// A configuration for building a cd image
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct CdConfiguration {
    /// Where to put the kernel image
    kernel_path: String,
}

#[enum_dispatch::enum_dispatch]
trait DiskBuilderTrait {
    /// Build the disk image
    fn build(
        &self,
        common: &DiskImageConfigurationCommon,
        kernel_machine: &str,
    ) -> Result<Disk, String>;
}

/// The configuration required to build an operating system disk image
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[enum_dispatch::enum_dispatch(DiskBuilderTrait)]
enum DiskImageConfigurationUnique {
    /// A bootable cd
    Cd(CdConfiguration),
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct DiskImageConfigurationCommon {
    /// Where to save the disk image
    output: std::path::PathBuf,
    /// What to label the disk image with
    disk_label: String,
    /// Generic files to put on the disk, along with their contents
    config_files: Vec<(std::path::PathBuf, String)>,
}

/// Defines the types of disks that can exist
pub enum Disk {
    /// A standard bootable cd
    Cd(std::path::PathBuf),
}

/// The configuration data required to build a disk image
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct DiskImageConfiguration {
    /// configuration data that varies between disk images
    unique: DiskImageConfigurationUnique,
    /// configuration data common to all disk images
    common: DiskImageConfigurationCommon,
}

impl DiskBuilderTrait for CdConfiguration {
    fn build(
        &self,
        common: &DiskImageConfigurationCommon,
        kernel_machine: &str,
    ) -> Result<Disk, String> {
        use std::io::Write;
        let cd_path = "./build/iso/boot/grub";
        std::fs::create_dir_all(cd_path).map_err(|e| e.to_string())?;

        let kernel = format!("./kernel/target/{}/release/kernel", kernel_machine);
        let new_kernel_path = std::path::PathBuf::from(&self.kernel_path);
        std::fs::copy(kernel, &new_kernel_path).map_err(|e| e.to_string())?;
        std::process::Command::new("strip")
            .arg(new_kernel_path)
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        for (fname, contents) in &common.config_files {
            let mut f = std::fs::File::create(fname).unwrap();
            f.write_all(contents.as_bytes()).unwrap();
        }

        let mut g = std::process::Command::new("grub-mkrescue");
        g.args([
            "-o",
            common
                .output
                .clone()
                .into_os_string()
                .into_string()
                .unwrap()
                .as_str(),
            "build/iso",
            "--",
            "-volid",
            &common.disk_label,
        ]);
        let cout = g
            .output()
            .expect("Failed to run command to build the kernel");
        if cout.status.success() {
            Ok(Disk::Cd(common.output.clone()))
        } else {
            Err(String::from_utf8(cout.stderr)
                .expect("Invalid output from cargo while building kernel"))
        }
    }
}

impl Default for DiskImageConfigurationUnique {
    fn default() -> Self {
        Self::Cd(CdConfiguration::default())
    }
}

/// The configuration for building doors
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct DoorsConfiguration {
    /// The configuration for the kernel
    kernel_config: config::KernelConfig,
    /// The machine name to use for building the kernel
    kernel_machine: String,
    /// The configuration required to build a disk image
    disk: DiskImageConfiguration,
    /// The target for running the final disk image
    target: Emulation,
}

impl DoorsConfiguration {
    /// Do something in the kernel directory
    fn in_kernel_dir<U>(&self, f: impl Fn() -> Result<U, String>) -> Result<U, String> {
        let olddir = std::env::current_dir().unwrap();
        let kernelpath =
            std::fs::canonicalize("./kernel").expect("Failed to build path for building kernel");
        std::env::set_current_dir(kernelpath).unwrap();
        let result = f();
        std::env::set_current_dir(olddir).unwrap();
        result
    }

    /// Build the kernel for the operating system
    pub fn build_kernel(&self) -> Result<(), String> {
        self.in_kernel_dir(|| {
            let mut c = std::process::Command::new("cargo");
            let target = &self.kernel_machine;
            let cargo = c.args(["+nightly", "build", "--release", "--target", target]);
            let cout = cargo
                .output()
                .expect("Failed to run command to build the kernel");
            if cout.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8(cout.stderr)
                    .expect("Invalid output from cargo while building kernel"))
            }
        })
    }

    /// Build the disassembly for the kernel
    pub fn build_kernel_disassembly(&self) -> Result<String, String> {
        self.in_kernel_dir(|| {
            let mut c = std::process::Command::new("cargo");
            let target = &self.kernel_machine;
            let cargo = c.args([
                "+nightly",
                "objdump",
                "--release",
                "--target",
                target,
                "--bin",
                "kernel",
                "-q",
                "--",
                "-d",
            ]);
            let cout = cargo
                .output()
                .expect("Failed to run command to build the kernel");
            if cout.status.success() {
                Ok(String::from_utf8(cout.stdout)
                    .expect("Invalid output from cargo while building kernel"))
            } else {
                Err(String::from_utf8(cout.stderr)
                    .expect("Invalid output from cargo while building kernel"))
            }
        })
    }

    /// Build the emulator config
    pub fn build_emulator_config(&self, disk: &Disk) {
        self.target.build_config(disk);
    }

    /// Run the emulator (if applicable)
    pub fn run_emulator(&self) {
        self.target.run();
    }

    /// Build a disk image for the operating system
    #[cfg(target_os = "windows")]
    pub fn build_image(&self) -> Result<Disk, String> {
        Err("Not implemented".into())
    }

    /// Build a disk image for the operating system on linux using grub-mkrescue
    #[cfg(target_os = "linux")]
    pub fn build_image(&self, kernel_machine: &str) -> Result<Disk, String> {
        self.disk.unique.build(&self.disk.common, kernel_machine)
    }
}

/// Open the doors config file from the specified file
fn open_config_file(f: std::path::PathBuf) -> Option<DoorsConfiguration> {
    use std::io::Read;
    if f.as_path().exists() {
        let mut config = std::fs::File::open(&f).expect("Failed to open kernel configuration");
        let mut config_contents = Vec::new();
        config
            .read_to_end(&mut config_contents)
            .expect("Failed to read kernel configuration");
        let config =
            String::from_utf8(config_contents).expect("Invalid contents in kernel configuration");
        let mconfig = toml::from_str::<DoorsConfiguration>(&config);
        if mconfig.is_err() {
            let mut p2 = f.clone();
            p2.pop();
            println!("Need to check in path {:?}", p2);
        }
        let config = mconfig.expect("Invalid kernel configuration");
        Some(config)
    } else {
        let mut p2 = f.clone();
        p2.pop();
        let read = p2.as_path().read_dir().unwrap();
        println!("Doors config not found, valid files in same path are as follows:");
        for f in read {
            if let Ok(entry) = f {
                println!("Entry {:?}", entry.path());
            }
        }
        None
    }
}

fn main() {
    use std::io::Write;
    let args = Args::parse();
    println!("I am groot {:?}", args);
    let mut config: DoorsConfiguration = if let Some(n) = args.name {
        open_config_file(n).unwrap()
    } else {
        DoorsConfiguration::default()
    };

    println!("Doors configuration: {:?}", config);

    if let Some(f) = args.save {
        config
            .disk
            .common
            .config_files
            .push(("asdf".into(), "fdsa".to_string()));
        let text = toml::to_string_pretty(&config).expect("Failed to create configuration file");
        let mut configf =
            std::fs::File::create(&f).expect("Failed to create operating system configuration");
        configf
            .write_all(text.as_bytes())
            .expect("Failed to save configuration file");
    } else {
        print!("Building kernel... ");
        std::io::stdout().flush().unwrap();
        config
            .build_kernel()
            .inspect_err(|e| {
                println!("Failed to build the kernel");
                print!("{}", e);
            })
            .unwrap();
        println!("Kernel built");

        print!("Producing disassembly for kernel... ");
        std::io::stdout().flush().unwrap();
        let d = config
            .build_kernel_disassembly()
            .inspect_err(|e| {
                println!("Failed to build the kernel disassembly");
                print!("{}", e);
            })
            .unwrap();
        println!("{} bytes generated", d.len());

        print!("Building disk image... ");
        std::io::stdout().flush().unwrap();
        let disk = config.build_image(&config.kernel_machine).unwrap();
        println!("done");
        println!("Running disk image on {:?}", config.target);
        config.target.build_config(&disk);
        if let Some(mut emulator) = config.target.run().unwrap() {
            let _ = emulator.wait();
        }
    }
}
