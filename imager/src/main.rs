use hadris_iso::{
    BootEntryOptions, BootOptions, BootSectionOptions, EmulationType, FileInput, FileInterchange,
    FormatOptions, IsoImage, PlatformId,
};
use std::path::PathBuf;

fn make_iso() -> Result<std::fs::File, ()> {
    let options = FormatOptions::new()
        .with_files(FileInput::from_fs(PathBuf::from("path/to/iso_root")).map_err(|_| ())?)
        .with_level(FileInterchange::NonConformant)
        .with_boot_options(BootOptions {
            write_boot_catalogue: true,
            default: BootEntryOptions {
                boot_image_path: "boot.img".to_string(),
                load_size: 4,
                emulation: EmulationType::NoEmulation,
                boot_info_table: true,
                grub2_boot_info: false,
            },
            entries: vec![(
                BootSectionOptions {
                    platform_id: PlatformId::UEFI,
                },
                BootEntryOptions {
                    boot_image_path: "uefi-boot.img".to_string(),
                    load_size: 0, // This means the size will be calculated
                    emulation: EmulationType::NoEmulation,
                    boot_info_table: false,
                    grub2_boot_info: false,
                },
            )],
        });
    let file = IsoImage::format_file(PathBuf::from("my_image.iso"), options).map_err(|_| ())?;
    Ok(file)
}

fn main() {
    make_iso();
}
