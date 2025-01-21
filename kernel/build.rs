use std::io::Read;

use ttf_parser::{colr::Painter, OutlineBuilder, RgbaColor};

use crate::config::KernelConfig;

mod config;

struct BitmapPainter {}

impl BitmapPainter {
    fn new() -> Self {
        Self {}
    }
}

impl OutlineBuilder for BitmapPainter {
    fn move_to(&mut self, x: f32, y: f32) {
        todo!()
    }

    fn line_to(&mut self, x: f32, y: f32) {
        todo!()
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        todo!()
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        todo!()
    }

    fn close(&mut self) {
        todo!()
    }
}

impl<'a> Painter<'a> for BitmapPainter {
    fn outline_glyph(&mut self, glyph_id: ttf_parser::GlyphId) {
        todo!();
    }

    fn paint(&mut self, paint: ttf_parser::colr::Paint<'a>) {
        todo!();
    }

    fn push_clip(&mut self) {
        todo!();
    }

    fn push_clip_box(&mut self, clipbox: ttf_parser::colr::ClipBox) {
        todo!();
    }

    fn pop_clip(&mut self) {
        todo!();
    }

    fn push_layer(&mut self, mode: ttf_parser::colr::CompositeMode) {
        todo!();
    }

    fn pop_layer(&mut self) {
        todo!();
    }

    fn push_transform(&mut self, transform: ttf_parser::Transform) {
        todo!();
    }

    fn pop_transform(&mut self) {
        todo!();
    }
}

fn generate_font_table(font: &[u8]) -> Vec<i32> {
    let font = ttf_parser::Face::parse(font, 0).unwrap();
    if font.is_variable() {
        panic!("Not a fixed width font");
    }
    let mut font_bitmap = Vec::new();
    let mut painter = BitmapPainter::new();
    let color = RgbaColor::new(255, 255, 255, 255);
    for i in 0..=255 {
        let c = i as u8 as char;
        if let Some(glyph) = font.glyph_index(c) {
            if font.is_color_glyph(glyph) {
                if font.paint_color_glyph(glyph, 0, color, &mut painter).is_none() {
                    panic!("Failed to paint color glyph {} {:?}", i, glyph);
                }
                font_bitmap.push(1);
            }
            else {
                font.outline_glyph(glyph, &mut painter).unwrap();
                font_bitmap.push(2);
            }
        } else {
            font_bitmap.push(0);
        }
    }
    font_bitmap
}

fn write_font_source(name: String, table: Vec<i32>) {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join(&name);
    let mut contents = String::new();

    contents.push_str("/// The generated fontmap\n");
    contents.push_str("pub static FONTMAP: &[u32] = &[");

    let mtable: Vec<String> = table.iter().map(|a| format!("0x{:x}", a)).collect();
    contents.push_str(&mtable.join(","));
    contents.push_str("];\n");

    std::fs::write(dest_path, contents).unwrap();
}

fn main() {
    let target = target_build_utils::TargetInfo::new().expect("could not get target info");

    println!("cargo:rerun-if-changed=./src/cmunbtl.ttf");
    let font = include_bytes!("./src/cmunbtl.ttf");
    let fontmap = generate_font_table(font);
    write_font_source("fontmap.rs".to_string(), fontmap);

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

    println!("cargo::rustc-check-cfg=cfg(kernel_machine, values(\"pc64\", \"stm32f769i-disco\"))");
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
