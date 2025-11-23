use anyhow::Result;
use clap::Parser;
use rustqlite::cli::Args;
use rustqlite::cli::Comando;
use rustqlite::page::PageHeader;
use rustqlite::page::PageType;
use rustqlite::page::SchemaCell;
use rustqlite::varint::read_varint;
use rustqlite::varint::read_varint_from_offset;
use std::fs::File;
use std::io::BufReader;
use std::io::Seek;
use std::io::prelude::Read;
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering;

static PAGE_SIZE: AtomicU16 = AtomicU16::new(4096);

fn main() -> Result<()> {
    let Args { cmd, db_path } = Args::parse();

    let mut file = File::open(&db_path)?;
    let mut header = [0; 100];
    file.read_exact(&mut header)?;

    // The page size is stored at the 16th byte offset, using 2 bytes in big-endian order
    PAGE_SIZE.store(
        u16::from_be_bytes([header[16], header[17]]),
        Ordering::Relaxed,
    );

    match cmd {
        Comando::DatabaseInfo => {
            println!("database page size: {}", PAGE_SIZE.load(Ordering::Relaxed));

            let mut sqlite_schema_header = [0; 12];
            file.read_exact(&mut sqlite_schema_header)?;
            let number_of_tables =
                u16::from_be_bytes([sqlite_schema_header[3], sqlite_schema_header[4]]);

            println!("number of tables: {number_of_tables}")
        }
        Comando::Tables => {
            // Read the database header.
            let page_size = PAGE_SIZE.load(Ordering::Relaxed);
            println!("database page size: {}", page_size);
            let mut file = BufReader::new(File::open(&db_path)?);
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
                    let rowid = read_varint(&mut file)?;

                    // First we read the payload.
                    let mut payload_bytes = vec![0; payload_size];
                    file.read_exact(&mut payload_bytes)?;

                    let schema_cell = SchemaCell::new(&payload_bytes)?;

                    let infos_offset = (schema_cell.rootpage - 1) * page_size as u64;

                    print!(
                        "id: {rowid}, cell name: {}, Start of data: {infos_offset}\n",
                        schema_cell.name
                    );
                }
                println!()
            }
        }
        Comando::Sql { query } => {
            println!("Basta rodar o comando: {query}")
        }
    }

    Ok(())
}
