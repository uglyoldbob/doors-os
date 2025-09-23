//! This utility is used to build a complete operating system

#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]

use std::{io::Write, path::PathBuf, str::FromStr};

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
    fn custom_debug_symbols(&self, _cmakelists: &mut String, _s: std::path::PathBuf) {}
    /// Build the config for the emulator
    fn build_config(
        &self,
        disk: &Disk,
        common: &EmulatorConfig,
        local: &LocalConfiguration,
        s: std::path::PathBuf,
    );
    /// Write rules to run the emulator
    fn run(
        &self,
        cmakelists: &mut String,
        common: &EmulatorConfig,
        local: &LocalConfiguration,
        s: std::path::PathBuf,
    );
}

/// An emulation target that does nothing
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct NoEmulator {}

impl EmulationTrait for NoEmulator {
    fn build_config(
        &self,
        _disk: &Disk,
        _common: &EmulatorConfig,
        _local: &LocalConfiguration,
        _s: std::path::PathBuf,
    ) {
    }

    fn run(
        &self,
        _cmakelist: &mut String,
        _common: &EmulatorConfig,
        _local: &LocalConfiguration,
        _s: std::path::PathBuf,
    ) {
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
    /// The items to use for serial port emulation
    pub serial_ports: Vec<usize>,
}

/// The holder of the emulation enum and the common configuration for all emulator types
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct EmulatorCommon {
    /// The actual emulation implementation
    pub emulator: Emulation,
    /// Common config for all emulators
    pub config: EmulatorConfig,
}

/// Command line arguments for the tool
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the configuration to use
    #[arg(short, long)]
    name: std::path::PathBuf,
}

/// A configuration for building a cd image
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct CdConfiguration {
    /// Where to put the kernel image
    kernel_path: String,
}

/// A configuration for building a cd image
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct NetworkConfiguration {
    /// Where in the tftp to copy the image to
    tftp_relative: std::path::PathBuf,
}

/// This trait is used to build disk images
#[enum_dispatch::enum_dispatch]
trait DiskBuilderTrait {
    /// Build the disk image
    fn build(
        &self,
        cmakelists: &mut String,
        common: &DiskImageConfigurationCommon,
        kernel_machine: &str,
        local: &LocalConfiguration,
    ) -> Result<Disk, String>;
    /// Fetch an existing disk image
    fn fetch(&self, common: &DiskImageConfigurationCommon) -> Result<Disk, String>;
    /// Add rules to deploy for disk images where it is applicable
    fn deploy(
        &self,
        local: &LocalConfiguration,
        common: &DiskImageConfigurationCommon,
        cmakelists: &mut String,
    );
}

/// The configuration required to build an operating system disk image
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[enum_dispatch::enum_dispatch(DiskBuilderTrait)]
enum DiskImageConfigurationUnique {
    /// A bootable cd
    Cd(CdConfiguration),
    /// A network bootable image
    Network(NetworkConfiguration),
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct DiskImageConfigurationCommon {
    /// Where to save the disk image
    output: std::path::PathBuf,
    /// What to label the disk image with
    disk_label: String,
    /// Generic files to put on the disk, source and destination names
    config_files: Vec<(String, String)>,
    /// Optional Grub config override for the source filename
    grub_config: Option<std::path::PathBuf>,
}

impl DiskImageConfigurationCommon {
    /// Get the pathbuf from the config_files member
    pub fn get_config_files(&self) -> Vec<(PathBuf, PathBuf)> {
        self.config_files
            .iter()
            .map(|a| {
                if cfg!(target_os = "windows") {
                    let s1 = a.0.clone();
                    let s2 = a.1.clone();
                    (
                        PathBuf::from(s1.replace("/", "\\")),
                        PathBuf::from(s2.replace("/", "\\")),
                    )
                } else {
                    (PathBuf::from(a.0.clone()), PathBuf::from(a.1.clone()))
                }
            })
            .collect()
    }
}

/// Defines the types of disks that can exist
#[derive(Debug)]
pub enum Disk {
    /// A standard bootable cd
    Cd(std::path::PathBuf),
    /// A pxe bootable network image
    Network(std::path::PathBuf),
}

/// The configuration data required to build a disk image
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct DiskImageConfiguration {
    /// configuration data that varies between disk images
    unique: DiskImageConfigurationUnique,
    /// configuration data common to all disk images
    common: DiskImageConfigurationCommon,
}

impl DiskBuilderTrait for NetworkConfiguration {
    fn deploy(
        &self,
        local: &LocalConfiguration,
        common: &DiskImageConfigurationCommon,
        cmakelists: &mut String,
    ) {
        let mut pa = local
            .tftp_base
            .clone()
            .expect("Local configuration is missing tftp_base");
        pa.push(&self.tftp_relative);
        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tboot_disk_deploy\n");
        cmakelists.push_str("\tDEPENDS boot_disk\n");

        if cfg!(target_os = "windows") {
            cmakelists.push_str(&format!("\tCOMMAND mkdir {}\n", pa.display(),));
        } else {
            cmakelists.push_str(&format!("\tCOMMAND mkdir -p {}\n", pa.display(),));
        }
        cmakelists.push_str(&format!(
            "\tCOMMAND cp -r {}/* {}\n",
            LocalConfiguration::escape_path(&common.output),
            pa.display(),
        ));
        cmakelists.push_str(")\n");
    }

    fn fetch(&self, common: &DiskImageConfigurationCommon) -> Result<Disk, String> {
        Ok(Disk::Network(common.output.clone()))
    }

    fn build(
        &self,
        cmakelists: &mut String,
        common: &DiskImageConfigurationCommon,
        kernel_path: &str,
        local: &LocalConfiguration,
    ) -> Result<Disk, String> {
        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tboot_disk\n");
        cmakelists.push_str("\tDEPENDS kernel\n");
        cmakelists.push_str(&format!(
            "\tBYPRODUCTS {}\n",
            common.output.to_str().unwrap()
        ));
        let pa = std::path::Path::new("./build/net/boot");
        if cfg!(target_os = "windows") {
            cmakelists.push_str(&format!("\tCOMMAND mkdir {}\n", pa.display(),));
        } else {
            cmakelists.push_str(&format!("\tCOMMAND mkdir -p {}\n", pa.display(),));
        }

        if cfg!(target_os = "windows") {
            todo!();
        } else if cfg!(target_os = "linux") {
            cmakelists.push_str(&format!(
                "\tCOMMAND cp {}/* ./build/net\n",
                local.grub_source.display()
            ));
            cmakelists.push_str(&format!(
                "\tCOMMAND grub-mknetdir --net-directory {} --subdir=/boot/grub -d ./build/net\n",
                LocalConfiguration::escape_path(&common.output),
            ));
        } else {
            panic!();
        }
        cmakelists.push_str(&format!(
            "\tCOMMAND cp ./kernel/target/{}/release/kernel {}/boot/kernel\n",
            kernel_path,
            LocalConfiguration::escape_path(&common.output),
        ));
        cmakelists.push_str(&format!(
            "\tCOMMAND strip {}/boot/kernel\n",
            LocalConfiguration::escape_path(&common.output),
        ));
        for (fname, dest) in common.get_config_files() {
            if cfg!(target_os = "windows") {
                cmakelists.push_str(&format!("\tCOMMAND copy {:?} {:?}\n", fname, dest));
            } else {
                cmakelists.push_str(&format!("\tCOMMAND cp {:?} {:?}\n", fname, dest));
            }
        }
        if let Some(s) = &common.grub_config {
            cmakelists.push_str(&format!(
                "\tCOMMAND cp {} {}/boot/grub/grub.cfg\n",
                LocalConfiguration::escape_path(&s),
                LocalConfiguration::escape_path(&common.output)
            ));
            cmakelists.push_str(")\n");
        } else {
            cmakelists.push_str(&format!(
                "\tCOMMAND cp grub2.lst {}/boot/grub/grub.cfg\n",
                LocalConfiguration::escape_path(&common.output)
            ));
            cmakelists.push_str(")\n");
        }
        Ok(Disk::Network(common.output.clone()))
    }
}

impl DiskBuilderTrait for CdConfiguration {
    fn deploy(
        &self,
        _local: &LocalConfiguration,
        _common: &DiskImageConfigurationCommon,
        _cmakelists: &mut String,
    ) {
    }

    fn fetch(&self, common: &DiskImageConfigurationCommon) -> Result<Disk, String> {
        Ok(Disk::Cd(common.output.clone()))
    }

    fn build(
        &self,
        cmakelists: &mut String,
        common: &DiskImageConfigurationCommon,
        kernel_path: &str,
        _local: &LocalConfiguration,
    ) -> Result<Disk, String> {
        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tboot_disk\n");
        cmakelists.push_str("\tDEPENDS kernel\n");
        cmakelists.push_str(&format!(
            "\tBYPRODUCTS {}\n",
            common.output.to_str().unwrap()
        ));
        if cfg!(target_os = "windows") {
            cmakelists.push_str("\tCOMMAND rmdir /s /q .\\\\build\\\\iso\n");
        } else {
            cmakelists.push_str("\tCOMMAND rm -rf ./build/iso\n");
        }
        let mut pa = std::path::PathBuf::from(".");
        pa.push("build");
        pa.push("iso");
        pa.push("boot");
        pa.push("grub");
        if cfg!(target_os = "windows") {
            cmakelists.push_str(&format!("\tCOMMAND mkdir {:?}\n", pa,));
            cmakelists.push_str(
                "\tCOMMAND copy grub2.lst .\\\\build\\\\iso\\\\boot\\\\grub\\\\grub.cfg\n",
            );
            cmakelists.push_str(&format!(
                "\tCOMMAND copy .\\\\kernel\\\\target\\\\{}\\\\release\\\\kernel .\\\\build\\\\iso\\\\boot\\\\kernel\n",
                kernel_path
            ));
        } else {
            cmakelists.push_str(&format!("\tCOMMAND mkdir -p {}\n", pa.display(),));
            cmakelists.push_str("\tCOMMAND cp grub2.lst ./build/iso/boot/grub/grub.cfg\n");
            cmakelists.push_str(&format!(
                "\tCOMMAND cp ./kernel/target/{}/release/kernel ./build/iso/boot/kernel\n",
                kernel_path
            ));
        }

        cmakelists.push_str("\tCOMMAND rust-strip ./build/iso/boot/kernel\n");
        for (fname, dest) in common.get_config_files() {
            if cfg!(target_os = "windows") {
                cmakelists.push_str(&format!("\tCOMMAND copy {:?} {:?}\n", fname, dest));
            } else {
                cmakelists.push_str(&format!("\tCOMMAND cp {:?} {:?}\n", fname, dest));
            }
        }

        if cfg!(target_os = "windows") {
            cmakelists
                .push_str("\tCOMMAND cargo +nightly run --bin imager -- --iso-path=build/iso\n");
        /*
        cmakelists.push_str(&format!(
            "\tCOMMAND {} createiso --import-iso grub-skeleton.iso -o {} --name-setup=iso9660 ./boot/kernel=./build/iso/boot/kernel --volid=\"{}\"\n",
            LocalConfiguration::escape_path(&local.vboximg_path()),
            LocalConfiguration::escape_path(&common.output),
            common.disk_label
        )); */
        } else if cfg!(target_os = "linux") {
            cmakelists.push_str(&format!(
                "\tCOMMAND grub-mkrescue -o {} build/iso -- -volid \"{}\"\n",
                LocalConfiguration::escape_path(&common.output),
                common.disk_label
            ));
        } else {
            panic!();
        }
        if cfg!(target_os = "windows") {
            cmakelists.push_str("\tCOMMAND rmdir /s /q .\\\\build\\\\iso\n");
        } else {
            cmakelists.push_str("\tCOMMAND rm -rf ./build/iso\n");
        }
        cmakelists.push_str(")\n");
        Ok(Disk::Cd(common.output.clone()))
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
    /// The machine name to use for building user programs
    user_machine: String,
    /// The output path for the machine used to build the kernel
    kernel_path: String,
    /// The configuration required to build a disk image
    disk: DiskImageConfiguration,
    /// The target for running the final disk image
    target: EmulatorCommon,
    /// Should the disassembly be created
    disassembly: bool,
}

/// The configuration for a qemu network
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct QemuNetworkConfig {
    /// The network type
    pub net_type: String,
    /// The device name
    pub dev_name: String,
}

/// A configuration for a single local network card used by an emulator
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct NetworkConfig {
    /// The configuration string for bochs
    bochs: Option<String>,
    /// The configuration string for qemu
    qemu: Option<QemuNetworkConfig>,
    /// The configuration string for virtualbox
    virtualbox: Option<String>,
}

/// A configuration for a single local network card used by an emulator
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum SerialConfig {
    /// The serial port output goes to a file
    File(std::path::PathBuf),
    /// The serial port is made available by a tcp server on the specified port
    TcpServer(u16),
    /// The serial port is made available by a tcp client on the specified port
    TcpClient(u16),
    /// A physical serial port
    Real(String),
    /// A non-existant serial port that goes nowhere
    Nothing,
}

/// Configuration specific to the build machine
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct LocalConfiguration {
    /// The binary for bochs
    bochs_path: Option<std::path::PathBuf>,
    /// The binary for qemu
    qemu_path: Option<std::path::PathBuf>,
    /// The binary for virtualbox
    virtualbox_path: Option<std::path::PathBuf>,
    /// The binary for vboxmanage, to manage virtualbox images
    vboxmanage_path: Option<std::path::PathBuf>,
    /// The binary for gdb
    gdb_path: Option<std::path::PathBuf>,
    /// Network devices that can be used by emulators
    pub net_devs: Vec<NetworkConfig>,
    /// Serial ports that can be used by emulators
    pub serial_ports: Vec<SerialConfig>,
    /// Optional base path the tftp server for deploying network images
    pub tftp_base: Option<std::path::PathBuf>,
    /// where to find the files for the grub command to make the netboot image
    grub_source: std::path::PathBuf,
    /// The target for the local system
    target: String,
}

impl Default for LocalConfiguration {
    fn default() -> Self {
        Self {
            target: "x86_64-unknown-linux-gnu".to_string(),
            bochs_path: Some(
                std::path::PathBuf::from_str("./optional/example/bochs/path/here").unwrap(),
            ),
            qemu_path: Some(
                std::path::PathBuf::from_str("./optional/example/qemu/path/here").unwrap(),
            ),
            virtualbox_path: Some(
                std::path::PathBuf::from_str("./optional/example/virtualbox/path/here").unwrap(),
            ),
            vboxmanage_path: Some(
                std::path::PathBuf::from_str("./optional/example/vboxmanage/path/here").unwrap(),
            ),
            gdb_path: Some(
                std::path::PathBuf::from_str("./optional/example/gdb/path/here").unwrap(),
            ),
            net_devs: vec![
                NetworkConfig {
                    bochs: Some("bochs_net_config".to_string()),
                    qemu: Some(QemuNetworkConfig {
                        net_type: "tap".to_string(),
                        dev_name: "tap0".to_string(),
                    }),
                    virtualbox: Some("virtualbox_net_config".to_string()),
                },
                NetworkConfig {
                    bochs: Some("bochs_net_config2".to_string()),
                    qemu: Some(QemuNetworkConfig {
                        net_type: "tap".to_string(),
                        dev_name: "tap1".to_string(),
                    }),
                    virtualbox: Some("virtualbox_net_config2".to_string()),
                },
            ],
            serial_ports: vec![
                SerialConfig::File(PathBuf::from_str("./example/serial/file.log").unwrap()),
                SerialConfig::TcpServer(1234),
                SerialConfig::TcpClient(1235),
                SerialConfig::Real("/dev/fakeport0".to_string()),
                SerialConfig::Nothing,
            ],
            tftp_base: Some(PathBuf::from_str("./example/tftp/base").unwrap()),
            grub_source: PathBuf::from_str("./example/grub/source").unwrap(),
        }
    }
}

impl LocalConfiguration {
    /// Put escapes into a path containing \ and " "
    fn escape_path(path: &std::path::Path) -> String {
        let a = path.to_str().unwrap().to_string();
        a.replace("\\", "\\\\").replace(" ", "\\ ")
    }

    #[cfg(target_os = "linux")]
    /// Get the path for the bochs binary
    pub fn bochs_path(&self) -> std::path::PathBuf {
        self.bochs_path.clone().unwrap_or("bochs".into())
    }

    #[cfg(target_os = "windows")]
    /// Get the path for the bochs binary
    pub fn bochs_path(&self) -> std::path::PathBuf {
        self.bochs_path
            .clone()
            .unwrap_or("C:\\Program Files\\Bochs-2.8\\bochsdbg.exe".into())
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
        self.vboxmanage_path
            .clone()
            .unwrap_or("C:\\Program Files\\Oracle\\VirtualBox\\VBoxManage.exe".into())
    }

    #[cfg(target_os = "linux")]
    /// Get the path for the vbox-img binary
    pub fn gdb_path(&self) -> std::path::PathBuf {
        self.gdb_path.clone().unwrap_or("gdb".into())
    }

    #[cfg(target_os = "windows")]
    /// Get the path for the vbox-img binary
    pub fn gdb_path(&self) -> std::path::PathBuf {
        self.gdb_path.clone().unwrap_or("gdb".into())
    }
}

impl DoorsConfiguration {
    /// Build the user code for the operating system
    pub fn build_user(&self, cmakelists: &mut String, target: &str) {
        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tuser\n");
        cmakelists.push_str("\tBYPRODUCTS target\n");
        cmakelists.push_str(&format!(
            "\tCOMMAND cargo build --release --target {}\n",
            target
        ));
        cmakelists.push_str(")\n");

        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tuser_clippy\n");
        cmakelists.push_str("\tBYPRODUCTS target\n");
        cmakelists.push_str(&format!("\tCOMMAND cargo clippy --target {}\n", target));
        cmakelists.push_str(")\n");

        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tuser_fmt\n");
        cmakelists.push_str("\tBYPRODUCTS target\n");
        cmakelists.push_str("\tCOMMAND cargo fmt\n");
        cmakelists.push_str(")\n");
    }

    /// Build the kernel for the operating system
    pub fn build_kernel(&self, cmakelists: &mut String, target: &str) {
        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tkernel\n");
        cmakelists.push_str("\tDEPENDS user\n");
        cmakelists.push_str("\tBYPRODUCTS target\n");
        cmakelists.push_str(&format!(
            "\tCOMMAND cargo +nightly build --release --target {} --bin kernel\n",
            target
        ));
        cmakelists.push_str(")\n");

        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tkernel_clippy\n");
        cmakelists.push_str("\tBYPRODUCTS target\n");
        cmakelists.push_str(&format!(
            "\tCOMMAND cargo +nightly clippy --target {}\n",
            target
        ));
        cmakelists.push_str(")\n");

        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tkernel_fmt\n");
        cmakelists.push_str("\tBYPRODUCTS target\n");
        cmakelists.push_str("\tCOMMAND cargo +nightly fmt\n");
        cmakelists.push_str(")\n");

        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tkernel_expand\n");
        cmakelists.push_str("\tBYPRODUCTS target\n");
        cmakelists.push_str("\tCOMMAND cargo +nightly expand > ../expanded.txt\n");
        cmakelists.push_str(")\n");
    }

    /// Build the disassembly for the kernel
    pub fn build_kernel_disassembly(&self, cmakelists: &mut String, target: &str) {
        cmakelists.push_str("add_custom_target(\n");
        cmakelists.push_str("\tdisassemble\n");
        cmakelists.push_str("\tDEPENDS kernel\n");
        cmakelists.push_str("\tBYPRODUCTS ../disassemble.txt\n");
        cmakelists.push_str(&format!("\tCOMMAND cargo objdump --release --target {} --bin kernel -q -- -d > ../disassemble.txt\n", target));
        cmakelists.push_str(")\n");
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

/// Build the cmakelists files from the configuration and the program arguments
fn build_cmake_files(_args: &Args, config: MasterConfig) {
    use std::io::Write;
    let mut cmakelist = String::new();
    let mut kernel_cmakelist = String::new();
    let mut user_cmakelist = String::new();

    let kernel_binary_path = std::path::PathBuf::from(format!(
        "./kernel/target/{}/release/kernel",
        config.os.kernel_path
    ));

    cmakelist.push_str("cmake_minimum_required(VERSION 3.22)\n");
    cmakelist.push_str("project(doors-os)\n");
    cmakelist.push_str("include(ExternalProject)\n");
    cmakelist.push_str("add_subdirectory(kernel)\n");
    cmakelist.push_str("add_subdirectory(user)\n");
    cmakelist.push_str(&format!("set(LOCAL_TARGET {})\n", config.local.target));
    cmakelist.push_str(&format!("set(USER_TARGET {})\n", config.os.user_machine));

    cmakelist.push_str(
        "configure_file(rust_bootstrap ./rust/bootstrap.toml)\n",
    );
    cmakelist.push_str(
        "configure_file(rust_compiler_toolchain.toml ./rust/rustup-toolchain.toml COPYONLY)\n",
    );

    cmakelist.push_str("add_custom_target(\n");
    cmakelist.push_str("\tfmt\n");
    cmakelist.push_str("\tDEPENDS kernel_fmt user_fmt\n");
    cmakelist.push_str("\tCOMMAND cargo fmt\n");
    cmakelist.push_str(")\n");

    cmakelist.push_str("add_custom_target(\n");
    cmakelist.push_str("\trust_compiler\n");
    cmakelist.push_str("\tWORKING_DIRECTORY ./rust\n");
    cmakelist.push_str("\tCOMMAND python ./x.py build\n");
    cmakelist.push_str("\tCOMMAND python ./x.py install\n");
    cmakelist.push_str(
        "\tCOMMAND rustup toolchain link doors-user ${CMAKE_CURRENT_BINARY_DIR}/rust-install\n",
    );
    cmakelist.push_str(")\n");

    cmakelist.push_str("add_custom_target(\n");
    cmakelist.push_str("\tstack\n");
    cmakelist.push_str("\tDEPENDS kernel\n");
    cmakelist.push_str(&format!(
        "\tCOMMAND cargo run --bin kernel-stack-analysis -- --name {}\n",
        kernel_binary_path.to_str().unwrap()
    ));
    cmakelist.push_str(")\n");

    kernel_cmakelist.push_str("cmake_minimum_required(VERSION 3.22)\n");
    kernel_cmakelist.push_str("project(doors-kernel)\n\n");

    write_kernel_config(&config);

    config
        .os
        .build_kernel(&mut kernel_cmakelist, &config.os.kernel_machine);
    config
        .os
        .build_kernel_disassembly(&mut kernel_cmakelist, &config.os.kernel_machine);

    config
        .os
        .build_user(&mut user_cmakelist, &config.os.user_machine);

    let disk = config
        .os
        .disk
        .unique
        .build(
            &mut cmakelist,
            &config.os.disk.common,
            &config.os.kernel_path,
            &config.local,
        )
        .unwrap();
    config
        .os
        .disk
        .unique
        .deploy(&config.local, &config.os.disk.common, &mut cmakelist);
    config.os.target.emulator.build_config(
        &disk,
        &config.os.target.config,
        &config.local,
        kernel_binary_path.clone(),
    );
    config.os.target.emulator.run(
        &mut cmakelist,
        &config.os.target.config,
        &config.local,
        kernel_binary_path.clone(),
    );
    config
        .os
        .target
        .emulator
        .custom_debug_symbols(&mut cmakelist, kernel_binary_path);

    {
        let mut configf = std::fs::File::create("./CMakeLists.txt")
            .expect("Failed to create cmake configuration");
        configf
            .write_all(cmakelist.as_bytes())
            .expect("Failed to save CMakeLists.txt file");
    }
    {
        let mut configf = std::fs::File::create("./kernel/CMakeLists.txt")
            .expect("Failed to create kernel cmake configuration");
        configf
            .write_all(kernel_cmakelist.as_bytes())
            .expect("Failed to save kernel CMakeLists.txt file");
    }
    {
        let mut configf = std::fs::File::create("./user/CMakeLists.txt")
            .expect("Failed to create user cmake configuration");
        configf
            .write_all(user_cmakelist.as_bytes())
            .expect("Failed to save user CMakeLists.txt file");
    }

    write_vscode_configs(&config);
}

/// Write the files used by vscode
fn write_vscode_configs(config: &MasterConfig) {
    use std::io::Write;
    print!("Writing vscode config...");
    std::io::stdout().flush().unwrap();
    {
        let p = std::path::PathBuf::from("./kernel/.vscode/settings.json");
        let mut p2 = p.clone();
        p2.pop();
        let _ = std::fs::create_dir_all(p2);
        let mut configf = std::fs::File::create(p).expect("Failed to create kernel configuration");
        let mut contents = String::new();
        contents.push_str("{\n");
        contents.push_str("\t\"rust-analyzer.cargo.allTargets\": false,\n");
        contents.push_str(&format!(
            "\t\"rust-analyzer.cargo.target\": \"{}\",\n",
            config.os.kernel_machine
        ));
        contents.push_str("}\n");
        configf
            .write_all(contents.as_bytes())
            .expect("Failed to save configuration file");
    }
    println!("done");
}

/// Write the config used directly by the kernel build process
fn write_kernel_config(config: &MasterConfig) {
    use std::io::Write;
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
}

fn main() {
    let args = Args::parse();
    let config: DoorsConfiguration = open_config_file(args.name.to_path_buf()).unwrap();
    let lc = open_local_config("./local_config.toml".into());
    let local = if let Some(lc) = lc {
        lc
    } else {
        let mut file = std::fs::File::create("./local_config.toml").unwrap();
        let lc = LocalConfiguration::default();
        let data = toml::to_string(&lc).unwrap();
        file.write_all(data.as_bytes()).unwrap();
        lc
    };

    let config = MasterConfig::build(local, config);
    build_cmake_files(&args, config);
}
