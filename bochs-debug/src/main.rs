use clap::Parser;
use object::ObjectSymbol;

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
    let mut syms = String::new();
    for s in o.symbols() {
        if let Ok(sn) = s.name() {
            syms.push_str(&format!(
                "{:x} {}\n",
                s.address(),
                rustc_demangle::demangle(sn)
            ));
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
