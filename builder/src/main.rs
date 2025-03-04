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
    /// Build any custom debug symbols required
    fn custom_debug_symbols(&self, _s: std::path::PathBuf) {}
    /// Build the config for the emulator
    fn build_config(&self, disk: &Disk, local: &LocalConfiguration);
    /// Run the emulator
    fn run(
        &self,
        local: &LocalConfiguration,
    ) -> Result<Option<std::process::Child>, std::io::Error>;
    /// Get the simple name of the emulator for build purposes
    fn simple_name(&self) -> &str;
}

/// An emulation target that does nothing
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct NoEmulator {}

impl EmulationTrait for NoEmulator {
    fn build_config(&self, _disk: &Disk, local: &LocalConfiguration) {}

    fn run(
        &self,
        _local: &LocalConfiguration,
    ) -> Result<Option<std::process::Child>, std::io::Error> {
        Ok(None)
    }

    fn simple_name(&self) -> &str {
        "none"
    }
}

/// Specifies how the operating system is to be emulated or run
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, strum::EnumIter)]
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

/// The basic modes of operation for this utility
#[derive(clap::ValueEnum, Clone, Parser, Debug)]
enum BuildMode {
    /// Construct a CMakeLists.txt file for running cmake
    Cmake,
    /// Run the build directly with this tool
    Build,
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
    #[arg(long)]
    mode: BuildMode,
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
        local: &LocalConfiguration,
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
        local: &LocalConfiguration,
    ) -> Result<Disk, String> {
        use std::io::Write;
        let cd_path = "./build/iso/boot/grub";
        std::fs::create_dir_all(cd_path).map_err(|e| e.to_string())?;

        let kernel = format!("./kernel/target/{}/release/kernel", kernel_machine);
        let new_kernel_path = std::path::PathBuf::from(&self.kernel_path);
        std::fs::copy(&kernel, &new_kernel_path).map_err(|e| e.to_string())?;
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

        let mut g = if cfg!(target_os = "windows") {
            let imgm = if let Some(p) = &local.vboximg_path {
                p.to_owned()
            } else {
                "vbox-img".into()
            };
            let mut imgm = std::process::Command::new(imgm);
            imgm.args([
                "createiso",
                "--import-iso",
                "grub-skeleton.iso",
                "-o",
                common.output.to_str().unwrap(),
                "--name-setup=iso9660",
                &format!("./boot/kernel={}", &kernel),
                &format!("--volid=\"{}\"", &common.disk_label),
            ]);
            imgm
        } else if cfg!(target_os = "linux") {
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
                "-o",
                common.output.to_str().unwrap(),
                "--",
                "-volid",
                &common.disk_label,
            ]);
            g
        } else {
            panic!();
        };
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

/// Configuration specific to the build machine
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct LocalConfiguration {
    /// The binary for bochs
    pub bochs_path: Option<std::path::PathBuf>,
    /// The binary for qemu
    pub qemu_path: Option<std::path::PathBuf>,
    /// The binary for virtualbox
    pub virtualbox_path: Option<std::path::PathBuf>,
    /// The binary for vboxmanage, to manage virtualbox images
    pub vboxmanage_path: Option<std::path::PathBuf>,
    /// The binary for vbox-img, to build images in certain situations
    pub vboximg_path: Option<std::path::PathBuf>,
}

impl Default for LocalConfiguration {
    #[cfg(target_os = "linux")]
    fn default() -> Self {
        Self {
            bochs_path: None,
            qemu_path: None,
            virtualbox_path: None,
            vboxmanage_path: None,
            vboximg_path: None,
        }
    }

    #[cfg(target_os = "windows")]
    fn default() -> Self {
        Self {
            bochs_path: Some("C:\\Program Files\\Bochs-2.8\\bochsdbg.exe".into()),
            qemu_path: Some("C:\\Program Files\\qemu\\qemu-system-x86_64.exe".into()),
            virtualbox_path: Some("C:\\Program Files\\Oracle\\VirtualBox\\VirtualBoxVM.exe".into()),
            vboxmanage_path: Some("C:\\Program Files\\Oracle\\VirtualBox\\VBoxManage.exe".into()),
            vboximg_path: Some("C:\\Program Files\\Oracle\\VirtualBox\\vbox-img.exe".into()),
        }
    }
}

impl DoorsConfiguration {
    /// Add rules to a cmakelists document
    fn make_cmake_rules(&self, cmakelist: &mut String, target: &str) {
        let rule = &format!(
            "add_custom_target(
    {0}
    BYPRODUCTS ./{0}.iso
    COMMAND cargo run --release --bin builder -- --mode build --name ./configs/{0}.toml
)
",
            target
        );
        cmakelist.push_str(rule);
    }

    /// Build the kernel for the operating system
    pub fn build_kernel(&self) -> Result<(), String> {
        let mut c = std::process::Command::new("cargo");
        let target = &self.kernel_machine;
        let cargo = c.args([
            "+nightly",
            "build",
            "--release",
            "--target",
            target,
            "--bin",
            "kernel",
        ]);
        cargo.current_dir("./kernel");
        let cout = cargo
            .output()
            .expect("Failed to run command to build the kernel");
        if cout.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8(cout.stderr)
                .expect("Invalid output from cargo while building kernel"))
        }
    }

    /// Build the disassembly for the kernel
    pub fn build_kernel_disassembly(&self) -> Result<String, String> {
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
        cargo.current_dir("./kernel");
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
    }

    /// Build a disk image for the operating system
    pub fn build_image(
        &self,
        kernel_machine: &str,
        local_config: &LocalConfiguration,
    ) -> Result<Disk, String> {
        self.disk
            .unique
            .build(&self.disk.common, kernel_machine, local_config)
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
        println!(
            "Doors config {} not found, valid files in same path are as follows:",
            f.display()
        );
        let read = p2.as_path().read_dir().unwrap();
        for entry in read.flatten() {
            println!("Entry {:?}", entry.path());
        }
        None
    }
}

/// Open the local build machine configuration
fn open_local_config(f: std::path::PathBuf) -> Option<LocalConfiguration> {
    use std::io::Read;
    if f.as_path().exists() {
        let mut config = std::fs::File::open(&f).expect("Failed to open local configuration");
        let mut config_contents = Vec::new();
        config
            .read_to_end(&mut config_contents)
            .expect("Failed to read kernel configuration");
        let config =
            String::from_utf8(config_contents).expect("Invalid contents in local configuration");
        let mconfig = toml::from_str::<LocalConfiguration>(&config);
        if mconfig.is_err() {
            let mut p2 = f.clone();
            p2.pop();
            println!("Need to check in path {:?}", p2);
        }
        let config = mconfig.expect("Invalid local configuration");
        Some(config)
    } else {
        let mut p2 = f.clone();
        p2.pop();
        let read = p2.as_path().read_dir().unwrap();
        println!("Local configuration not found, valid files in same path are as follows:");
        for entry in read.flatten() {
            println!("Entry {:?}", entry.path());
        }
        None
    }
}

/// Combined configuration structure of both local and operating system configuration
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct MasterConfig {
    /// The local configuration
    local: LocalConfiguration,
    /// The operating system config
    os: DoorsConfiguration,
}

impl MasterConfig {
    /// Construct a new Self from its members
    pub fn build(local: LocalConfiguration, os: DoorsConfiguration) -> Self {
        Self { local, os }
    }
}

fn add_to_cmakelist(cmakelist: &mut String, f: &std::path::PathBuf, target: &str) {
    let config = open_config_file(f.to_path_buf()).unwrap();
    let local = open_local_config("./local_config.toml".into()).unwrap_or_default();
    let config = MasterConfig::build(local, config);
    config.os.make_cmake_rules(cmakelist, target);
}

fn build_cmake_files(_args: &Args, _config: MasterConfig) {
    use std::io::Write;
    let p = std::path::PathBuf::from("./configs");
    let read = p.as_path().read_dir().unwrap();
    println!("Cmake files:");
    let mut cmakelist = String::new();
    cmakelist.push_str("cmake_minimum_required(VERSION 3.22)\n");
    cmakelist.push_str("project(doors-os)\n\n");

    for entry in read.flatten() {
        if let Ok(ft) = entry.file_type() {
            if ft.is_file() {
                if entry.file_name().to_str().unwrap().ends_with(".toml") {
                    let f = entry.path();
                    if let Some(name) = f.file_stem() {
                        add_to_cmakelist(&mut cmakelist, &f, name.to_str().unwrap());
                    }
                }
            }
        }
    }

    let mut configf =
        std::fs::File::create("./CMakeLists.txt").expect("Failed to create cmake configuration");
    configf
        .write_all(cmakelist.as_bytes())
        .expect("Failed to save CMakeLists.txt file");
}

fn run_build(args: &Args, mut config: MasterConfig) {
    use std::io::Write;
    if let Some(f) = &args.save {
        config
            .os
            .disk
            .common
            .config_files
            .push(("asdf".into(), "fdsa".to_string()));
        let text = toml::to_string_pretty(&config.os).expect("Failed to create configuration file");
        let mut configf =
            std::fs::File::create(f).expect("Failed to create operating system configuration");
        configf
            .write_all(text.as_bytes())
            .expect("Failed to save configuration file");
    } else {
        print!("Writing kernel config...");
        std::io::stdout().flush().unwrap();
        {
            let mut configf = std::fs::File::create("./kernel/config.toml")
                .expect("Failed to create kernel configuration");
            let text = toml::to_string_pretty(&config.os.kernel_config)
                .expect("Failed to create kernel configuration file");
            configf
                .write_all(text.as_bytes())
                .expect("Failed to save configuration file");
        }
        println!("done");

        print!("Building kernel... ");
        std::io::stdout().flush().unwrap();
        config
            .os
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
            .os
            .build_kernel_disassembly()
            .inspect_err(|e| {
                println!("Failed to build the kernel disassembly");
                print!("{}", e);
            })
            .unwrap();
        {
            let mut configf = std::fs::File::create("./disassemble.txt")
                .expect("Failed to create disassembly file");
            configf
                .write_all(d.as_bytes())
                .expect("Failed to save disassembly file");
        }
        println!("{} bytes generated", d.len());

        print!("Building disk image... ");
        std::io::stdout().flush().unwrap();
        let disk = config
            .os
            .build_image(&config.os.kernel_machine, &config.local)
            .unwrap();
        println!("done");
        println!("Running disk image on {:?}", config.os.target);
        config.os.target.custom_debug_symbols(
            format!(
                "./kernel/target/{}/release/kernel",
                &config.os.kernel_machine
            )
            .into(),
        );
        config.os.target.build_config(&disk, &config.local);
        if let Some(mut emulator) = config.os.target.run(&config.local).unwrap() {
            let _ = emulator.wait();
        }
    }
}

fn main() {
    let args = Args::parse();
    let config: DoorsConfiguration = if let Some(n) = &args.name {
        open_config_file(n.to_path_buf()).unwrap()
    } else {
        DoorsConfiguration::default()
    };
    let local = open_local_config("./local_config.toml".into()).unwrap_or_default();
    let config = MasterConfig::build(local, config);
    match args.mode {
        BuildMode::Cmake => {
            build_cmake_files(&args, config);
        }
        BuildMode::Build => {
            run_build(&args, config);
        }
    }
}
