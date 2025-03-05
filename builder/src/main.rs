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
    fn build_config(&self, disk: &Disk, common: &EmulatorConfig, local: &LocalConfiguration);
    /// Run the emulator
    fn run(
        &self,
        common: &EmulatorConfig,
        local: &LocalConfiguration,
    ) -> Result<Option<std::process::Child>, std::io::Error>;
    /// Get the simple name of the emulator for build purposes
    fn simple_name(&self) -> &str;
}

/// An emulation target that does nothing
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct NoEmulator {}

impl EmulationTrait for NoEmulator {
    fn build_config(&self, _disk: &Disk, _common: &EmulatorConfig, _local: &LocalConfiguration) {}

    fn run(
        &self,
        _common: &EmulatorConfig,
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

/// Common config for emulators
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct EmulatorConfig {
    /// The nodes to use for network devices in the emulator
    pub net_devs: Vec<usize>,
}

/// The holder of the emulation enum and the common configuration for all emulator types
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct EmulatorCommon {
    /// The actual emulation implementation
    pub emulator: Emulation,
    /// Common config for all emulators
    pub config: EmulatorConfig,
}

/// The basic modes of operation for this utility
#[derive(clap::ValueEnum, Clone, Parser, Debug)]
enum BuildMode {
    /// Construct a CMakeLists.txt file for running cmake
    Cmake,
    /// Run the build directly with this tool
    Build,
    /// Run the build and then run the emulator
    BuildAndRun,
    /// Just run the emulator
    Run,
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
    /// Fetch an existing disk image
    fn fetch(&self, common: &DiskImageConfigurationCommon) -> Result<Disk, String>;
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
#[derive(Debug)]
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
    fn fetch(&self, common: &DiskImageConfigurationCommon) -> Result<Disk,String> {
        Ok(Disk::Cd(common.output.clone()))
    }

    fn build(
        &self,
        common: &DiskImageConfigurationCommon,
        kernel_path: &str,
        local: &LocalConfiguration,
    ) -> Result<Disk, String> {
        use std::io::Write;
        let cd_path = "./build/iso/boot/grub";
        std::fs::create_dir_all(cd_path).map_err(|e| e.to_string())?;

        let kernel = format!("./kernel/target/{}/release/kernel", kernel_path);
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
            let mut imgm = std::process::Command::new(local.vboximg_path());
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
            .expect("Failed to run command to build the cd image");
        if cout.status.success() {
            Ok(Disk::Cd(common.output.clone()))
        } else {
            Err(String::from_utf8(cout.stderr)
                .expect("Invalid output from cargo while building cd image"))
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
    /// The output path for the machine used to build the kernel
    kernel_path: String,
    /// The configuration required to build a disk image
    disk: DiskImageConfiguration,
    /// The target for running the final disk image
    target: EmulatorCommon,
    /// Should the disassembly be created
    disassembly: bool,
}

/// Configuration specific to the build machine
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct LocalConfiguration {
    /// The binary for bochs
    bochs_path: Option<std::path::PathBuf>,
    /// The binary for qemu
    qemu_path: Option<std::path::PathBuf>,
    /// The binary for virtualbox
    virtualbox_path: Option<std::path::PathBuf>,
    /// The binary for vboxmanage, to manage virtualbox images
    vboxmanage_path: Option<std::path::PathBuf>,
    /// The binary for vbox-img, to build images in certain situations
    vboximg_path: Option<std::path::PathBuf>,
    /// Network devices that can be used by emulators
    pub net_devs: Vec<String>,
}

impl LocalConfiguration {
    #[cfg(target_os = "linux")]
    /// Get the path for the bochs binary
    pub fn bochs_path(&self) -> std::path::PathBuf {
        self.bochs_path.clone().unwrap_or("bochs".into())
    }

    #[cfg(target_os = "windows")]
    /// Get the path for the bochs binary
    pub fn bochs_path(&self) -> std::path::PathBuf {
        self.bochs_path.clone().unwrap_or("C:\\Program Files\\Bochs-2.8\\bochsdbg.exe".into())
    }

    #[cfg(target_os = "linux")]
    /// Get the path for the qemu binary
    pub fn qemu_path(&self) -> std::path::PathBuf {
        self.qemu_path
            .clone()
            .unwrap_or("qemu-system-x86_64".into())
    }

    #[cfg(target_os = "windows")]
    /// Get the path for the qemu binary
    pub fn qemu_path(&self) -> std::path::PathBuf {
        self.qemu_path
            .clone()
            .unwrap_or("C:\\Program Files\\qemu\\qemu-system-x86_64.exe".into())
    }

    #[cfg(target_os = "linux")]
    /// Get the path for the virtualbox binary
    pub fn virtualbox_path(&self) -> std::path::PathBuf {
        self.virtualbox_path
            .clone()
            .unwrap_or("VirtualBoxVM".into())
    }

    #[cfg(target_os = "windows")]
    /// Get the path for the virtualbox binary
    pub fn virtualbox_path(&self) -> std::path::PathBuf {
        self.virtualbox_path
            .clone()
            .unwrap_or("C:\\Program Files\\Oracle\\VirtualBox\\VirtualBoxVM.exe".into())
    }

    #[cfg(target_os = "linux")]
    /// Get the path for the vboxmanage binary
    pub fn vboxmanage_path(&self) -> std::path::PathBuf {
        self.vboxmanage_path.clone().unwrap_or("VBoxManage".into())
    }

    #[cfg(target_os = "windows")]
    /// Get the path for the vboxmanage binary
    pub fn vboxmanage_path(&self) -> std::path::PathBuf {
        self.vboxmanage_path.clone().unwrap_or("C:\\Program Files\\Oracle\\VirtualBox\\VBoxManage.exe".into())
    }

    #[cfg(target_os = "linux")]
    /// Get the path for the vbox-img binary
    pub fn vboximg_path(&self) -> std::path::PathBuf {
        self.vboximg_path.clone().unwrap_or("vbox-img".into())
    }

    #[cfg(target_os = "windows")]
    /// Get the path for the vbox-img binary
    pub fn vboximg_path(&self) -> std::path::PathBuf {
        self.vboximg_path.clone().unwrap_or("C:\\Program Files\\Oracle\\VirtualBox\\vbox-img.exe".into())
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
            .output();
        match cout {
            Ok(cout) => {
                if cout.status.success() {
                    Ok(())
                } else {
                    Err(String::from_utf8(cout.stderr)
                        .expect("Invalid output from cargo while building kernel"))
                }
            }
            Err(e) => {
                Err(e.to_string())
            }
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
            "--",
            "-d",
        ]);
        cargo.current_dir("./kernel");
        let cout = cargo
            .output()
            .expect("Failed to run command to disassemble the kernel");
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
        kernel_path: &str,
        local_config: &LocalConfiguration,
    ) -> Result<Disk, String> {
        self.disk
            .unique
            .build(&self.disk.common, kernel_path, local_config)
    }

    /// Fetch the disk image for the operating system
    pub fn fetch_image(&self) -> Result<Disk, String> {
        self.disk.unique.fetch(&self.disk.common)
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

fn run_build(args: &Args, config: &mut MasterConfig) {
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
        let kernel = config
            .os
            .build_kernel()
            .inspect_err(|e| {
                println!("Failed to build the kernel");
                print!("{}", e);
            });
        match kernel {
            Ok(_) => println!("Kernel built"),
            Err(e) => {
                println!("Failed to build\n{}", e);
                panic!();
            }
        }

        if config.os.disassembly {
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
        }

        print!("Building disk image... ");
        std::io::stdout().flush().unwrap();
        config
            .os
            .build_image(&config.os.kernel_path, &config.local)
            .unwrap();
        println!("done");
        config.os.target.emulator.custom_debug_symbols(
            format!(
                "./kernel/target/{}/release/kernel",
                &config.os.kernel_path
            )
            .into(),
        );
    }
}

fn run_emulator(_args: &Args, config: &MasterConfig) {
    let disk = config
            .os
            .fetch_image()
            .unwrap();
    println!(
            "Running disk image {:?} on {}",
            disk,
            config.os.target.emulator.simple_name()
        );
    config
            .os
            .target
            .emulator
            .build_config(&disk, &config.os.target.config, &config.local);
        if let Some(mut emulator) = config
            .os
            .target
            .emulator
            .run(&config.os.target.config, &config.local)
            .unwrap()
        {
            let _ = emulator.wait();
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
    let mut config = MasterConfig::build(local, config);
    match args.mode {
        BuildMode::Cmake => {
            build_cmake_files(&args, config);
        }
        BuildMode::Build => {
            run_build(&args, &mut config);
        }
        BuildMode::Run => {
            run_emulator(&args, &config);
        }
        BuildMode::BuildAndRun => {
            run_build(&args, &mut config);
            run_emulator(&args, &config);
        }
    }
}
