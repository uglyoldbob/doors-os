use std::io::{Read, Write};

use clap::Parser;

/// Command line arguments for the tool
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the binary to analyze
    #[arg(short, long)]
    name: std::path::PathBuf,
}

fn main() {
    let args = Args::parse();
    let mut f = std::fs::File::open(args.name).unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    let functions = stack_sizes::analyze_executable(&buf).unwrap();
    let mut o = std::fs::File::create("./stack.txt").unwrap();

    let functions_unsorted = functions.defined.clone();
    let mut fcollec: Vec<(u64, stack_sizes::Function)> = functions_unsorted.into_iter().collect();
    fcollec.sort_by_key(|a| a.1.stack());

    for (address, f) in fcollec.into_iter().rev() {
        let fname = f
            .names()
            .into_iter()
            .map(|a| rustc_demangle::demangle(*a).as_str());
        for name in fname {
            o.write_all(format!("Function {} @{:x}\n", name, address).as_bytes())
                .unwrap();
            if let Some(stack) = f.stack() {
                o.write_all(
                    format!("\t size: 0x{:x}, stack: 0x{:x}\n", f.size(), stack).as_bytes(),
                )
                .unwrap();
            } else {
                o.write_all(format!("\t size: 0x{:x}, stack: None\n", f.size()).as_bytes())
                    .unwrap();
            }
        }
    }
}
