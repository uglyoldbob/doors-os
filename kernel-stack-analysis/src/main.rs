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
    for (address, f) in functions.defined {
        let fname = f.names().into_iter().map(|a| { rustc_demangle::demangle(*a).as_str() });
        for name in fname {
            o.write_all(format!("Function {} @{:x} {:x?}\n", name, address, f).as_bytes()).unwrap();    
        }
    }
}
