use anyhow::{Result, bail};
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::Read;
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering;

mod page;
mod varint;

use page::{PageHeader, PageType, SchemaCell};
use varint::{read_varint, read_varint_from_offset};

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
        ".tables" => {
            // Read the database header.
            let mut file = BufReader::new(File::open(&args[1])?);
            let mut db_header = [0; 100];
            PAGE_SIZE.store(
                u16::from_be_bytes([db_header[16], db_header[17]]),
                Ordering::Relaxed,
            );
            file.read_exact(&mut db_header)?;

            // Read first page header.
            //
            // The header could occuppy a total of 12 bytes if it's a interior node of the tree. 8
            // if it's a leaf node.
            let mut header_bytes = [0; 12];
            file.read_exact(&mut header_bytes)?;
            let page_header = PageHeader::new(header_bytes.as_slice())?;
            if matches!(page_header.ty, PageType::Leaf(..)) {
                file.seek_relative(-4)?;
            }

            // Read cell pointer array.
            //
            // The cells are ordered by key size. Lefmost cell has the key with smallest number,
            // rightmost with the bigger number.
            let mut cell_ptr_arr_bytes = vec![0; page_header.number_of_cells as usize * 2];
            file.read_exact(&mut cell_ptr_arr_bytes)?;

            // HACK: Avoid reallocation?
            let cell_ptr_arr = cell_ptr_arr_bytes
                .chunks_exact(2)
                .map(|offset| u16::from_be_bytes([offset[0], offset[1]]))
                .collect::<Vec<_>>();

            // Visit cells and print the table name.
            //
            // Assuming that sqlite_shchema is always a leaf table.
            if matches!(page_header.ty, PageType::Leaf(..)) {
                for offset in cell_ptr_arr {
                    // Page 1, it's safe to directly go to the offset.
                    let payload_size = read_varint_from_offset(&mut file, offset as u64)? as usize;

                    // Assuming that's always a rowid table.
                    let _rowid = read_varint(&mut file)?;

                    // First we read the payload.
                    let mut payload_bytes = vec![0; payload_size];
                    file.read_exact(&mut payload_bytes)?;

                    let schema_cell = SchemaCell::new(&payload_bytes)?;
                    print!("{} ", schema_cell.tbl_name)
                }
                println!()
            }
        }
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}
