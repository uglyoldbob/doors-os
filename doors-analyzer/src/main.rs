use crate::os::OperatingSystemTrait;
use clap::Parser;
use object::{Object, ObjectSection, ObjectSegment};
use regex::bytes::{CaptureMatches, Captures};
use std::io::Read;

mod os;

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
    /// The activity to do with the operating system
    #[arg(short, long)]
    activity: String,
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
            return Ok(Self::Elf(o));
        }
        Ok(Self::Raw(c))
    }

    /// Get a byte at the specified address
    pub fn get_u8(&self, index: usize) -> Option<u8> {
        match self {
            DumpFile::Elf(file) => {
                for s in file.segments() {
                    if (s.address()..s.address() + s.size()).contains(&(index as u64)) {
                        let a = s.data().unwrap();
                        return Some(a[index - s.address() as usize]);
                    }
                }
                for s in file.sections() {
                    if (s.address()..s.address() + s.size()).contains(&(index as u64)) {
                        let a = s.data().unwrap();
                        return Some(a[index - s.address() as usize]);
                    }
                }
                None
            }
            DumpFile::Raw(items) => items.get(index).copied(),
        }
    }

    /// Get a slice at the specified location, of the specified length
    pub fn get_slice_with_length(&self, start: usize, len: usize) -> Option<Vec<u8>> {
        let range = start..start + len;
        self.get_slice(range)
    }

    /// Get a slice at the specified location
    pub fn get_slice(&self, range: std::ops::Range<usize>) -> Option<Vec<u8>> {
        match self {
            DumpFile::Elf(file) => {
                for s in file.segments() {
                    if (s.address()..s.address() + s.size()).contains(&(range.start as u64))
                        && (s.address()..s.address() + s.size()).contains(&(range.end as u64))
                    {
                        let a = s.data().unwrap();
                        let new_range =
                            range.start - s.address() as usize..range.end - s.address() as usize;
                        return a.get(new_range).map(|a| a.to_vec());
                    }
                }
                for s in file.sections() {
                    if (s.address()..s.address() + s.size()).contains(&(range.start as u64))
                        && (s.address()..s.address() + s.size()).contains(&(range.end as u64))
                    {
                        let a = s.data().unwrap();
                        let new_range =
                            range.start - s.address() as usize..range.end - s.address() as usize;
                        return a.get(new_range).map(|a| a.to_vec());
                    }
                }
                None
            }
            DumpFile::Raw(items) => items.get(range).map(|a| a.to_vec()),
        }
    }

    /// Find a slice using a regex
    pub fn find_with_regex(&self, r: regex::bytes::Regex) -> Vec<Captures> {
        match self {
            DumpFile::Elf(file) => {
                let mut combined = Vec::new();
                let a = file.segments().map(|s| {
                    let a = s.data().unwrap();
                    let b: Vec<Captures> = r.captures_iter(a).collect();
                    b
                });
                let b = file.sections().map(|s| {
                    let a = s.data().unwrap();
                    let b: Vec<Captures> = r.captures_iter(a).collect();
                    b
                });
                for mut a in a {
                    combined.append(&mut a);
                }
                for mut a in b {
                    combined.append(&mut a);
                }
                combined
            }
            DumpFile::Raw(items) => r.captures_iter(items).collect(),
        }
    }

    /// Find a slice within a slice
    pub fn find_subslice(&self, needle: &[u8]) -> Option<usize> {
        if needle.is_empty() {
            return None;
        }
        match self {
            DumpFile::Elf(file) => {
                for s in file.segments() {
                    let a = s.data().unwrap();
                    if let Some(d) = a.windows(needle.len()).position(|window| window == needle) {
                        return Some(d + s.address() as usize);
                    }
                }
                for s in file.sections() {
                    let a = s.data().unwrap();
                    if let Some(d) = a.windows(needle.len()).position(|window| window == needle) {
                        return Some(d + s.address() as usize);
                    }
                }
                None
            }
            DumpFile::Raw(items) => items
                .windows(needle.len())
                .position(|window| window == needle),
        }
    }
}

fn main() {
    let args = Args::parse();

    let mut f = std::fs::File::open(args.dump).unwrap();
    let mut contents = Vec::new();
    f.read_to_end(&mut contents).unwrap();

    let dump = DumpFile::auto_detect(&contents).unwrap();
    let kernel_contents = {
        let mut f = std::fs::File::open(args.kernel_symbols).unwrap();
        let mut contents = Vec::new();
        f.read_to_end(&mut contents).unwrap();
        contents
    };
    let kernel = object::File::parse(kernel_contents.as_slice()).unwrap();
    println!("Byte at 0x100000 is 0x{:02x?}", dump.get_u8(0x100000));
    println!(
        "Byte at 0x100000 is 0x{:02x?}",
        dump.get_slice(0x100000..0x101000)
    );

    let osd = os::OperatingSystemDetector::detect_os(&dump, &kernel);
    println!("OS detected is {:x?}", osd);
    if let Some(os) = osd {
        os.activity(&dump, &kernel, &args.activity);
    }
}
