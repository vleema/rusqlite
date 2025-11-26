mod cli;
mod database;
mod record;
mod varint;

use std::fs::File;

use anyhow::Ok;
use anyhow::{Result, bail};
use clap::Parser;

use cli::Args;
use cli::Cmd;
use database::CellInfo;
use database::Database;
use database::Page;
use record::SchemaCell;

fn main() -> Result<()> {
    let Args {
        cmd,
        db_path,
        query,
    } = Args::parse();

    let file = File::open(&db_path)?;
    let db = Database::open(&file)?;

    match cmd {
        Some(Cmd::DatabaseInfo) => {
            println!("database page size: {}", db.page_size);
            if let Page::Leaf { common } = db.get_page(1) {
                println!("number of tables: {}", common.cell_count);
            } else {
                todo!()
            }
        }
        Some(Cmd::Tables) => {
            if let pg @ Page::Leaf { .. } = db.get_page(1) {
                for offset in pg.cell_offset_list() {
                    let CellInfo::Leaf { payload, .. } = pg.parse_cell(offset) else {
                        unreachable!()
                    };
                    let schema_cell = SchemaCell::new(payload);
                    print!("{} ", schema_cell.tbl_name)
                }
                println!();
            } else {
                todo!()
            }
        }
        None => {
            let Some(query) = query else {
                bail!("no command or query provided")
            };
            let Some(tbl_name) = query.split_ascii_whitespace().last() else {
                bail!("invalid sql query")
            };

            if let pg @ Page::Leaf { .. } = db.get_page(1) {
                for offset in pg.cell_offset_list() {
                    let CellInfo::Leaf { payload, .. } = pg.parse_cell(offset) else {
                        unreachable!()
                    };
                    let schema_cell = SchemaCell::new(payload);
                    if schema_cell.tbl_name == tbl_name
                        && let Page::Leaf { common } = db.get_page(schema_cell.rootpage)
                    {
                        println!("{}", common.cell_count);
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
