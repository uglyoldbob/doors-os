use std::io::Read;

doors_macros::define_config!();
use config::KernelConfig;

fn main() {
    let p = std::path::PathBuf::from("../../kernel/config.toml");
    let mut config = std::fs::File::open(p).expect("Failed to open kernel configuration");
    let mut config_contents = Vec::new();
    config
        .read_to_end(&mut config_contents)
        .expect("Failed to read kernel configuration");
    let config =
        String::from_utf8(config_contents).expect("Invalid contents in kernel configuration");
    let config = toml::from_str::<KernelConfig>(&config).expect("Invalid kernel configuration");

    let mut kernel_machine = String::new();
    kernel_machine.push_str("cargo::rustc-check-cfg=cfg(kernel_machine, values(");
    let all_machines: Vec<String> = ["pc32", "pc64", "stm32f769i-disco"]
        .iter()
        .map(|a| format!("\"{}\"", a))
        .collect();
    let all_machines_str = all_machines.join(",");
    kernel_machine.push_str(&all_machines_str);
    kernel_machine.push_str("))");
    println!("{}", kernel_machine);
    println!("cargo:rustc-cfg=kernel_machine=\"{}\"", config.machine_name);

    let linker_script = match config.machine_name.as_str() {
        "stm32f769i-disco" => Some("linker/arm/stm32f769i-disco.ld"),
        "pc32" | "pc64" => Some("linker/x86/linker.ld"),
        _ => {
            panic!("Unknown machine name {}", config.machine_name);
        }
    };

    let linker_script = linker_script.expect("Failed to get linker script definition");

    let mut linker_script_check = "../".to_string();
    linker_script_check.push_str(&linker_script);
    if !std::path::PathBuf::from(linker_script_check.to_string()).exists() {
        panic!("Linker script {} does not exist", linker_script);
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=config.toml");
    println!("cargo:rustc-link-arg=-T{}", linker_script);
    println!("cargo:rerun-if-changed={}", linker_script);
}
