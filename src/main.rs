use anyhow::{Result, bail};
use std::fs::File;
use std::io::prelude::Read;
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering;

mod page;
mod varint;

static PAGE_SIZE: AtomicU16 = AtomicU16::new(4096);

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    match args.len() {
        0 | 1 => bail!("Missing <database path> and <command>"),
        2 => bail!("Missing <command>"),
        _ => {}
    }

    let command = &args[2];
    match command.as_str() {
        ".dbinfo" => {
            let mut file = File::open(&args[1])?;
            let mut header = [0; 100];
            file.read_exact(&mut header)?;

            // The page size is stored at the 16th byte offset, using 2 bytes in big-endian order
            PAGE_SIZE.store(
                u16::from_be_bytes([header[16], header[17]]),
                Ordering::Relaxed,
            );

            println!("database page size: {}", PAGE_SIZE.load(Ordering::Relaxed));

            let mut sqlite_schema_header = [0; 12];
            file.read_exact(&mut sqlite_schema_header)?;
            let number_of_tables =
                u16::from_be_bytes([sqlite_schema_header[3], sqlite_schema_header[4]]);

            println!("number of tables: {number_of_tables}")
        }
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}
