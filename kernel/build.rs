use std::io::Read;

use fontdue::FontSettings;

doors_macros::define_config!();
use config::KernelConfig;

struct FontData {
    c: char,
    width: u8,
    height: u8,
    left: i8,
    top: i8,
    bounds_xmin: u8,
    bounds_ymin: u8,
    bounds_xmax: u8,
    bounds_ymax: u8,
    data: Vec<u8>,
}

fn generate_font_table(font: &[u8]) -> Vec<FontData> {
    let font = fontdue::Font::from_bytes(
        font,
        FontSettings {
            collection_index: 0,
            scale: 40.0,
            load_substitutions: false,
        },
    )
    .unwrap();
    let mut font_bitmap = Vec::new();
    for (c, _d) in font.chars() {
        let (a, b) = font.rasterize(*c, 20.0);
        let fd = FontData {
            c: *c,
            width: a.width as u8,
            height: a.height as u8,
            left: a.ymin as i8,
            top: a.xmin as i8,
            bounds_xmin: a.bounds.xmin as u8,
            bounds_ymin: a.bounds.ymin as u8,
            bounds_xmax: (a.bounds.xmin + a.bounds.width) as u8,
            bounds_ymax: (a.bounds.ymin + a.bounds.height) as u8,
            data: b.clone(),
        };
        font_bitmap.push(fd);
    }
    font_bitmap
}

fn write_font_source(name: String, table: Vec<FontData>) {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join(&name);
    let mut contents = String::new();

    contents.push_str("lazy_static! {\n");

    contents.push_str("/// The generated fontmap\n");
    contents.push_str("pub static ref FONTMAP: alloc::collections::BTreeMap<char, FontData> = {\n");
    contents.push_str("\tlet mut t = alloc::collections::BTreeMap::new();\n");

    let mtable = table.iter().map(|a| {
        let d: Vec<String> = a.data.iter().map(|n| format!("{}", n)).collect();
        let s = format!(
            "FontData {{ width: {}, 
    height: {}, 
    left: {}, 
    top: {}, 
    bounds_xmin: {}, 
    bounds_ymin: {}, 
    bounds_xmax: {}, 
    bounds_ymax: {},
    data: &[{}],}}",
            a.width,
            a.height,
            a.left,
            a.top,
            a.bounds_xmin,
            a.bounds_ymin,
            a.bounds_xmax,
            a.bounds_ymax,
            d.join(", ")
        );
        (a.c, s)
    });
    for e in mtable {
        contents.push_str(&format!("\tt.insert({:?}, {});\n", e.0, e.1));
    }

    contents.push_str("t\n");
    contents.push_str("};\n");
    contents.push_str("}\n");

    std::fs::write(dest_path, contents).unwrap();
}

fn main() {
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
        "stm32f769i-disco" => Some("src/boot/arm/stm32f769i-disco.ld"),
        "pc32" | "pc64" => Some("src/boot/x86/linker.ld"),
        _ => {
            panic!("Unknown machine name {}", config.machine_name);
        }
    };

    let linker_script = linker_script.expect("Failed to get linker script definition");

    if !std::path::PathBuf::from(linker_script.to_string()).exists() {
        panic!("Linker script {} does not exist", linker_script);
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=config.toml");
    println!("cargo:rustc-link-arg=-T{}", linker_script);
    println!("cargo:rerun-if-changed={}", linker_script);
}
