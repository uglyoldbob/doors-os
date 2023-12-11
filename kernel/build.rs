use std::io::Read;

use crate::config::KernelConfig;

mod config;

fn main() {
    let target = target_build_utils::TargetInfo::new().expect("could not get target info");

    let p = std::path::PathBuf::from("./config.toml");
    let mut config = std::fs::File::open(p).expect("Failed to open kernel configuration");
    let mut config_contents = Vec::new();
    config
        .read_to_end(&mut config_contents)
        .expect("Failed to read kernel configuration");
    let config =
        String::from_utf8(config_contents).expect("Invalid contents in kernel configuration");
    let config = toml::from_str::<KernelConfig>(&config).expect("Invalid kernel configuration");

    if config.get_arch() != target.target_arch() {
        panic!(
            "Invalid arch {} instead of {} specified for kernel build",
            target.target_arch(),
            config.get_arch()
        );
    }

    let mut linker_script = None;

    match config.machine_name.as_str() {
        "stm32f769i-disco" => {
            linker_script = Some("kernel/src/boot/arm/stm32f769i-disco.ld");
        }
        _ => {
            panic!("Unknown machine name {}", config.machine_name);
        }
    }

    let linker_script = linker_script.expect("Failed to get linker script definition");

    if !std::path::PathBuf::from(format!("../{}", linker_script)).exists() {
        panic!("Linker script {} does not exist", linker_script);
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-arg=-T{}", linker_script);
    println!("cargo:rerun-if-changed={}", linker_script);
}
