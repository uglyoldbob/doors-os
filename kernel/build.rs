fn main() {
    let target = target_build_utils::TargetInfo::new().expect("could not get target info");

    let mut linker_script = None;
    match target.target_arch() {
        "arm" => {
            linker_script = Some("kernel/src/boot/arm/linker.ld");
            println!("arm detected");
        }
        "x86" | "x86_64" => {
            linker_script = Some("kernel/src/boot/x86/linker.ld");
        }
        _ => {
            panic!("Other arch {} detected", target.target_arch());
        }
    }

    let linker_script = linker_script.expect("Failed to get linker script definition");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-arg=-T{}", linker_script);
    println!("cargo:rerun-if-changed={}", linker_script);
}
