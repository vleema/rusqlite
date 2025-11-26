mod cli;
mod database;
mod record;
mod varint;

use std::fs::File;

use anyhow::Ok;
use anyhow::Result;
use clap::Parser;

use cli::Args;
use cli::Cmd;
use database::Database;
use database::Page;
use record::SchemaCell;
use varint::read_varint;

fn main() -> Result<()> {
    let Args { cmd, db_path } = Args::parse();

    let file = File::open(&db_path)?;
    let db = Database::new(&file)?;

    match cmd {
        Cmd::DatabaseInfo => {
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
                for &offset in cell_offset_list.iter() {
                    let mut cursor = &db.mmap[offset as usize..];
                    let (payload_size, _) = read_varint(&mut cursor);
                    let (_, _) = read_varint(&mut cursor); // rowid.

                    let payload_bytes = &cursor[..payload_size as usize];

                    let schema_cell = SchemaCell::new(payload_bytes);
                    print!("{} ", schema_cell.tbl_name)
                }
                println!();
            } else {
                todo!()
            }
        }
        Cmd::Sql { query } => {
            if let Page::Leaf {
                cell_offset_list, ..
            } = db.get_page(1)
            {
                let schemas: Vec<SchemaCell<'_>> = cell_offset_list
                    .iter()
                    .map(|&offset| {
                        let mut cursor = &db.mmap[offset as usize..];
                        let (payload_size, _) = read_varint(&mut cursor);
                        let (_, _) = read_varint(&mut cursor); // rowid.

                        let payload_bytes = &cursor[..payload_size as usize];

                        SchemaCell::new(payload_bytes)
                    })
                    .collect();

                let schema = schemas.iter().find(|i| i.name == query).unwrap();

                let page = db.get_page(schema.rootpage);

                let cells = page.get_cells();

                for cell in cells {
                    println!("{cell:?}")
                }
            } else {
                todo!()
            }

            println!("Basta rodar o comando: {query}")
        }
    }

    Ok(())
}
