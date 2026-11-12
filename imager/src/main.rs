use clap::Parser;
use flate2::read::GzDecoder;
use hadris_iso::boot::EmulationType;
use hadris_iso::boot::options::{BootEntryOptions, BootOptions};
use hadris_iso::read::PathSeparator;
use hadris_iso::write::options::{BaseIsoLevel, CreationFeatures, IsoFormatOptions};
use hadris_iso::write::{InputTree, IsoImageWriter};
use std::fs::File;
use std::num::NonZeroU16;
use tar::Archive;

// The reader lives here for now, nothing in the image creation path uses it.
#[allow(dead_code)]
mod iso9660;

/// Command line arguments for the tool
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the configuration to use
    #[arg(short, long)]
    iso_path: std::path::PathBuf,
    /// The iso file that is written
    #[arg(short, long, default_value = "my_image.iso")]
    output: std::path::PathBuf,
    /// The volume id written into the iso
    #[arg(long, default_value = "TESTISO")]
    volid: String,
}

/// Extract the prebuilt grub2 bootloader files
fn extract_grub2_prebuilt(path: &str) -> Result<(), std::io::Error> {
    let tar_gz = File::open(path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(".")?;
    Ok(())
}

/// Turn the configured disk label into a volume id the iso format allows
///
/// The volume identifier is written with the D character set, which only allows `A-Z`, `0-9` and
/// `_`. Labels like `DoorsOS` are therefore uppercased and any other character becomes `_`, with a
/// warning so the change is not silent.
fn iso_volume_id(volid: &str) -> String {
    let mut id: String = volid
        .chars()
        .map(|c| {
            let u = c.to_ascii_uppercase();
            if u.is_ascii_uppercase() || u.is_ascii_digit() || u == '_' {
                u
            } else {
                '_'
            }
        })
        .collect();
    if id.as_str() != volid {
        log::warn!("Volume id \"{volid}\" written as \"{id}\"");
    }
    if id.len() > 32 {
        log::warn!("Volume id \"{id}\" truncated to 32 characters");
        id.truncate(32);
    }
    id
}

/// Build the bootable iso file
///
/// The el torito boot image is the grub cdboot image staged at the root of `iso_path` by the
/// build script, see the `boot_disk` rule it generates.
fn make_iso(
    iso_path: &std::path::Path,
    output: &std::path::Path,
    volid: &str,
) -> Result<(), String> {
    let files = InputTree::from_fs(iso_path, PathSeparator::ForwardSlash)
        .map_err(|e| format!("Failed to read {}: {e}", iso_path.display()))?;

    let boot = BootOptions {
        write_boot_catalog: true,
        default: BootEntryOptions {
            boot_image_path: "grub-eltorito.img".to_string(),
            // Grub's cdboot is loaded as 4 virtual sectors and reads the rest of the core image
            // itself, so this has to stay 4 instead of being derived from the image size.
            load_size: Some(NonZeroU16::new(4).unwrap()),
            emulation: EmulationType::NoEmulation,
            boot_info_table: false,
            grub2_boot_info: true,
        },
        // A second entry for the same image is not possible, the bios image cannot boot an uefi
        // system. An uefi image would have to be staged as well for that.
        entries: vec![],
    };

    let options = IsoFormatOptions {
        volume_name: iso_volume_id(volid),
        system_id: None,
        volume_set_id: None,
        publisher_id: None,
        preparer_id: None,
        application_id: None,
        sector_size: 2048,
        path_separator: PathSeparator::ForwardSlash,
        features: CreationFeatures {
            filenames: BaseIsoLevel::Level1 {
                supports_lowercase: true,
                // The rock ridge entries are only written when the base level claims them.
                supports_rrip: true,
            },
            // Grub's module names (normal.mod, part_macos.mod, ...) do not fit the 8.3 upper case
            // names of a plain iso 9660 image. Rock Ridge would keep them, but grub's iso9660
            // filesystem only honors the suspend records when the image contains an `ER` record,
            // which hadris does not write yet. Joliet is always understood, so use it together
            // with lower case names; this is also what the linux iso made by grub mkrescue relies
            // on for its longer names.
            joliet: Some(hadris_iso::joliet::JolietLevel::Level3),
            rock_ridge: Some(Default::default()),
            el_torito: Some(boot),
            ..Default::default()
        },
        strict_charset: false,
    };

    let mut buffer = std::io::Cursor::new(Vec::new());
    IsoImageWriter::create(&mut buffer, files, options)
        .map_err(|e| format!("Failed to write {}: {e}", output.display()))?;
    std::fs::write(output, buffer.into_inner())
        .map_err(|e| format!("Failed to write {}: {e}", output.display()))?;
    Ok(())
}

fn main() {
    let args = Args::parse();
    simple_logger::SimpleLogger::new().init().unwrap();
    if let Err(e) = make_iso(&args.iso_path, &args.output, &args.volid) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
