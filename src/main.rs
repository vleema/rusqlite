mod database;
mod record;
mod varint;

use std::fs::File;

use anyhow::Ok;
use anyhow::{Result, bail};

use database::Database;
use database::Page;
use record::SchemaCell;

use varint::read_varint;

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    match args.len() {
        0 | 1 => bail!("Missing <database path> and <command>"),
        2 => bail!("Missing <command>"),
        _ => {}
    }

    let file = File::open(&args[1])?;
    let db = Database::new(&file)?;

    let command = &args[2];
    match command.as_str() {
        ".dbinfo" => {
            println!("database page size: {}", db.page_size);
            if let Page::Leaf { cell_count, .. } = db.get_page(1) {
                println!("number of tables: {}", cell_count);
            } else {
                todo!()
            }
        }
        Cmd::Tables => {
            if let Page::Leaf {
                cell_offset_list, ..
            } = db.get_page(1)
            {
                for bs in cell_offset_list.chunks_exact(2) {
                    let offset = u16::from_be_bytes([bs[0], bs[1]]);

                    let mut cursor = &db.mmap[offset as usize..];
                    let (payload_size, payload_int_size) = read_varint(&mut cursor);
                    let (_, _) = read_varint(&mut cursor); // rowid.
                    let payload_body_size = payload_size - (payload_int_size) as i64;
                    let payload_bytes = &cursor[..payload_body_size as usize];

                    let schema_cell = SchemaCell::new(payload_bytes);
                    print!("{} ", schema_cell.tbl_name)
                }
                println!();
            } else {
                todo!()
            }
        }
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}
