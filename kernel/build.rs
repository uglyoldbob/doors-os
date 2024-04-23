use std::io::Read;

use ttf_parser::colr::Painter;

use crate::config::KernelConfig;

mod config;

struct BitmapPainter {}

impl BitmapPainter {
    fn new() -> Self {
        Self {}
    }
}

impl Painter for BitmapPainter {
    fn outline(&mut self, glyph_id: ttf_parser::GlyphId) {
        todo!()
    }

    fn paint_foreground(&mut self) {
        todo!()
    }

    fn paint_color(&mut self, color: ttf_parser::RgbaColor) {
        todo!()
    }
}

fn main() {
    let target = target_build_utils::TargetInfo::new().expect("could not get target info");

    println!("cargo:rerun-if-changed=./src/cmunbtl.ttf");
    let font = include_bytes!("./src/cmunbtl.ttf");
    let font = ttf_parser::Face::parse(font, 0).unwrap();
    if font.is_variable() {
        panic!("Not a fixed width font");
    }
    let mut font_bitmap = Vec::new();
    let mut painter = BitmapPainter::new();
    for i in 0..=255 {
        let c = i as u8 as char;
        if let Some(glyph) = font.glyph_index(c) {
            font.paint_color_glyph(glyph, 0, &mut painter);
            font_bitmap.push(1);
        } else {
            font_bitmap.push(0);
        }
    }

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

    println!("cargo:rustc-cfg=kernel_machine=\"{}\"", config.machine_name);

    match config.machine_name.as_str() {
        "stm32f769i-disco" => {
            linker_script = Some("kernel/src/boot/arm/stm32f769i-disco.ld");
        }
        "pc64" => {
            linker_script = Some("kernel/src/boot/x86/linker.ld");
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
