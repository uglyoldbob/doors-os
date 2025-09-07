use clap::Parser;
use gimli::EndianReader;

/// Command line arguments for the tool
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the binary to generate bochs symbols for
    #[arg(short, long)]
    binary: std::path::PathBuf,
    /// The output file for bochs symbols
    #[arg(short, long)]
    output: std::path::PathBuf,
}

fn make_symbols(s: std::path::PathBuf, output: std::path::PathBuf) {
    use std::io::{Read, Write};
    let mut f = std::fs::File::open(s).unwrap();
    let mut contents = Vec::new();
    f.read_to_end(&mut contents).unwrap();
    let o = object::File::parse(&*contents).unwrap();
    use object::Object;
    use object::ObjectSection;
    let d = gimli::Dwarf::load(|id| {
        let secname = id.name();
        let data = o
            .section_by_name_bytes(secname.as_bytes())
            .ok_or(())
            .map(|d| d.data());
        match data {
            Ok(Ok(d)) => Ok::<EndianReader<gimli::LittleEndian, &[u8]>, ()>(EndianReader::new(
                d,
                gimli::LittleEndian,
            )),
            _ => Ok::<EndianReader<gimli::LittleEndian, &[u8]>, ()>(EndianReader::new(
                &[],
                gimli::LittleEndian,
            )),
        }
    });
    let mut syms = String::new();
    if let Ok(d) = d {
        let mut dunit = d.units();
        while let Some(header) = dunit.next().unwrap() {
            // Parse the abbreviations and other information for this compilation unit.
            let unit = d.unit(header).unwrap();

            // Iterate over all of this compilation unit's entries.
            let mut entries = unit.entries();
            while let Some((_, entry)) = entries.next_dfs().unwrap() {
                // If we find an entry for a function, print it.
                if entry.tag() == gimli::DW_TAG_subprogram {
                    let mut attrs = entry.attrs();
                    let mut fdata = String::new();
                    let mut pc_low = None;
                    while let Ok(Some(attr)) = attrs.next() {
                        fdata
                            .push_str(&format!("Found a function with attr: {:x?}\n", attr.name()));
                        if attr.name() == gimli::DW_AT_low_pc {
                            if let gimli::read::AttributeValue::Addr(a) = attr.value() {
                                fdata.push_str(&format!("pc low is {:x}\n", a));
                                pc_low = Some(a)
                            }
                        }
                        if let Some(pc_low) = pc_low {
                            if attr.name() == gimli::DW_AT_name {
                                let v = attr.value();
                                if let gimli::read::AttributeValue::DebugStrRef(a) = v {
                                    let n2 = d.string(a).unwrap();
                                    let name = std::str::from_utf8(&n2).unwrap().to_string();
                                    fdata.push_str(&format!(
                                        "Linkage name ref is {} @ {:x}\n",
                                        name, pc_low
                                    ));
                                    if pc_low != 0 {
                                        syms.push_str(&format!("{:x} {}\n", pc_low, name));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let mut configf =
        std::fs::File::create(output).expect("Failed to create bochs debug symbols file");
    configf
        .write_all(syms.as_bytes())
        .expect("Failed to save bochs debug symbols file");
}

fn main() {
    let args = Args::parse();
    make_symbols(args.binary, args.output);
}
