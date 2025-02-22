use clap::Parser;
use rand::RngCore;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the network interface to use
    #[arg(short, long)]
    name: Option<String>,

    /// Number of packets to listen for
    #[arg(short, long, default_value_t = 10)]
    listen_count: usize,

    /// Number of random packets to send
    #[arg(short, long, default_value_t = 0)]
    random_count: usize,
}

fn main() {
    let args = Args::parse();
    let d = pcap::Device::list();
    let mut interfaces = Vec::new();
    if let Ok(list) = d {
        for d in list {
            let m = if let Some(devmatch) = &args.name {
                &d.name == devmatch }
            else {
                true
            };
            if m {
                println!("Device:");
                println!("\t {:?}", d);
                println!();
                interfaces.push(d);
            }
        }
    }
    if args.name.is_some() {
        for d in interfaces {
            let e = d.open();
            let mut mac_address = [0u8; 6];
            match e {
                Ok(mut e) => {
                    for _ in 0..args.listen_count {
                        let p = e.next_packet();
                        match p {
                            Ok(p) => {
                                println!("Got packet {:x?}", p);
                                mac_address.copy_from_slice(&p.data[6..12]);
                            }
                            Err(e) => println!("Error receiving packet {:?}", e),
                        }
                    }
                    for _ in 0..args.random_count {
                        let mut buf = vec![0;64];
                        let mut rng = rand::rng();
                        rng.fill_bytes(&mut buf);
                        (&mut buf[0..6]).copy_from_slice(&mac_address);
                        e.sendpacket(buf).unwrap();
                    }
                }
                Err(e) => {
                    println!("Failed to open capture {:?}", e);
                }
            }
        }
    }
}
