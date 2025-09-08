use clap::Parser;
use flate2::read::GzDecoder;
use hadris_iso::{
    BootEntryOptions, BootOptions, BootSectionOptions, EmulationType, FileInput, FileInterchange,
    FormatOptions, IsoImage, PartitionOptions, PlatformId,
};
use std::fs::File;
use std::path::PathBuf;
use tar::Archive;

mod iso9660;

/// Command line arguments for the tool
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the configuration to use
    #[arg(short, long)]
    iso_path: std::path::PathBuf,
}

/// Extract the prebuilt grub2 bootloader files
fn extract_grub2_prebuilt(path: &str) -> Result<(), std::io::Error> {
    let tar_gz = File::open(path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(".")?;
    Ok(())
}

/// Build the bootable iso file
fn make_iso(iso_path: std::path::PathBuf) -> Result<std::fs::File, ()> {
    let options = FormatOptions::new()
        .with_files(FileInput::from_fs(iso_path).map_err(|_| ())?)
        .with_level(FileInterchange::NonConformant)
        .with_format_options(PartitionOptions::GPT)
        .with_boot_options(BootOptions {
            write_boot_catalogue: true,
            default: BootEntryOptions {
                boot_image_path: "grub-eltorito.img".to_string(),
                load_size: 0,
                emulation: EmulationType::NoEmulation,
                boot_info_table: true,
                grub2_boot_info: true,
            },
            entries: vec![(
                BootSectionOptions {
                    platform_id: PlatformId::UEFI,
                },
                BootEntryOptions {
                    boot_image_path: "grub-eltorito.img".to_string(),
                    load_size: 0, // This means the size will be calculated
                    emulation: EmulationType::NoEmulation,
                    boot_info_table: true,
                    grub2_boot_info: true,
                },
            )],
        });
    let file = IsoImage::format_file(PathBuf::from("my_image.iso"), options).map_err(|_| ())?;
    Ok(file)
}

fn main() {
    let args = Args::parse();
    simple_logger::SimpleLogger::new().init().unwrap();
    make_iso(args.iso_path).unwrap();
}
