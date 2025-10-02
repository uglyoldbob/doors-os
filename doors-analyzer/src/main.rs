use std::io::Read;
use clap::Parser;
use object::{Object, ObjectSymbol};

/// Command line arguments for the tool
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the file to use as the memory dump
    #[arg(short, long)]
    dump: std::path::PathBuf,
    /// Name of the file to use for kernel symbols
    #[arg(short, long)]
    kernel_symbols: std::path::PathBuf,
}

enum DumpFile<'a> {
    Elf(object::File<'a>),
    Raw(&'a [u8]),
}

impl<'a> DumpFile<'a> {
    /// Try to detect the type of dump file based on the contents
    pub fn auto_detect(c: &'a [u8]) -> Result<Self, String> {
        if let Ok(o) = object::File::parse(c) {
            println!("Got a valid elf format dump");
            return Ok(Self::Elf(o))
        }
        Ok(Self::Raw(c))
    }

    /// Get a byte at the specified address
    pub fn get_u8(&self, index: usize) -> Option<u8> {
        match self {
            DumpFile::Elf(file) => todo!(),
            DumpFile::Raw(items) => {
                items.get(index).copied()
            }
        }
    }

    /// Get a slice at the specified location
    pub fn get_slice(&self, range: std::ops::Range<usize>) -> Option<Vec<u8>> {
        match self {
            DumpFile::Elf(file) => todo!(),
            DumpFile::Raw(items) => {
                items.get(range).map(|a|a.to_vec())
            }
        }
    }

    /// Find a slice within a slice
    pub fn find_subslice(&self, needle: &[u8]) -> Option<usize> {
        if needle.is_empty() {
            return None;
        }
        match self {
            DumpFile::Elf(file) => todo!(),
            DumpFile::Raw(items) => {
                items.windows(needle.len())
                .position(|window| window == needle)
            }
        }
    }
}

fn main() {
    let args = Args::parse();

    let mut f = std::fs::File::open(args.dump).unwrap();
    let mut contents = Vec::new();
    f.read_to_end(&mut contents).unwrap();

    let dump = DumpFile::auto_detect(&contents).unwrap();

    let banner_search = dump.find_subslice("DoorsOsIdentifier".as_bytes()).unwrap();

    let kernel_contents = {
        let mut f = std::fs::File::open(args.kernel_symbols).unwrap();
        let mut contents = Vec::new();
        f.read_to_end(&mut contents).unwrap();
        contents
    };
    let kernel = object::File::parse(kernel_contents.as_slice()).unwrap();
    
    let banner_symbol = kernel.symbol_by_name("KERNEL_STRING").unwrap();
    let banner_address = dump.get_slice(banner_symbol.address() as usize..banner_symbol.address() as usize+8).unwrap();
    let mut ba_buf = [0; 8];
    ba_buf.copy_from_slice(&banner_address[..]);
    let banner_address = usize::from_le_bytes(ba_buf);

    if banner_address == banner_search {
        println!("The kernel identifier is in the right location, at 0x{:x}", banner_address);
    }
}
