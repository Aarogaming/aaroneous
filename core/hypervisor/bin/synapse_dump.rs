use std::env;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        println!("Usage: synapse_dump <synapse_file> [--raw]");
        println!("\nDumps the contents of a .synapse shared memory file.");
        println!("  --raw    Show raw hex dump instead of parsed fields");
        std::process::exit(1);
    }

    let path = &args[0];
    let raw_mode = args.get(1).map_or(false, |a| a == "--raw");

    if let Err(e) = dump_synapse(path, raw_mode) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn dump_synapse(path: &str, raw_mode: bool) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Read the full file
    let mut data = Vec::new();
    reader.read_to_end(&mut data)?;

    let file_size = data.len();
    println!("Synapse Dump: {}", path);
    println!("File size: {} bytes ({:.2} KB)", file_size, file_size as f64 / 1024.0);
    println!();

    if raw_mode {
        println!("Raw hex dump:");
        println!("Offset    00 01 02 03 04 05 06 07  08 09 0A 0B 0C 0D 0E 0F  ASCII");
        println!("--------  -----------------------------------------------  ----------------");

        for (i, chunk) in data.chunks(16).enumerate() {
            let offset = i * 16;
            let hex: Vec<String> = chunk.iter()
                .map(|b| format!("{:02x}", b))
                .collect();
            let hex_str = if chunk.len() > 8 {
                format!("{}  {}", hex[..8].join(" "), hex[8..].join(" "))
            } else {
                format!("{:<41}", hex.join(" "))
            };
            let ascii: String = chunk.iter()
                .map(|&b| if b >= 0x20 && b < 0x7f { b as char } else { '.' })
                .collect();
            println!("{:08x}  {}  {}", offset, hex_str, ascii);
        }
    } else {
        // Parse synapse structure
        if data.len() < 16 {
            println!("File too small to be a valid synapse (< 16 bytes)");
            return Ok(());
        }

        // Check for magic bytes
        let magic = &data[..4];
        match magic {
            b"\xAA\x55\xAA\x55" => {
                println!("Magic: AAS Synapse (\\xAA\\x55\\xAA\\x55)");
            }
            b"TEL1" => {
                println!("Magic: Telemetry (TEL1)");
            }
            _ => {
                println!("Magic: {:02x} {:02x} {:02x} {:02x} (unknown)", magic[0], magic[1], magic[2], magic[3]);
            }
        }

        // Parse header fields
        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let status = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let field3 = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);

        println!("Version: {}", version);
        println!("Status: {}", status);
        println!("Field 3: {} (0x{:08x})", field3, field3);

        // Show payload size
        if data.len() > 16 {
            let payload_size = data.len() - 16;
            println!("Payload: {} bytes", payload_size);

            // Show first 256 bytes of payload as hex
            let show_bytes = payload_size.min(256);
            println!("\nFirst {} bytes of payload:", show_bytes);
            for (i, chunk) in data[16..16+show_bytes].chunks(16).enumerate() {
                let hex: Vec<String> = chunk.iter().map(|b| format!("{:02x}", b)).collect();
                let ascii: String = chunk.iter()
                    .map(|&b| if b >= 0x20 && b < 0x7f { b as char } else { '.' })
                    .collect();
                println!("  {:04x}: {:<48} {}", i * 16, hex.join(" "), ascii);
            }
        }
    }

    Ok(())
}
