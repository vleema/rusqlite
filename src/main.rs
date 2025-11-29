use std::fs::File;

use anyhow::Context;
use anyhow::Ok;
use anyhow::Result;
use clap::Parser;

mod btree;
mod cli;
mod record;
mod varint;
mod vm;

use btree::Database;
use btree::Entry;
use cli::Args;
use cli::Cmd;
use record::Schema;
use record::SerialType;
use varint::read_varint;

fn main() -> Result<()> {
    let Args { cmd, db_path, query } = Args::parse();

    let file = File::open(&db_path)?;
    let db = Database::open(&file)?;

    match cmd {
        Some(Cmd::DatabaseInfo) => {
            println!("database page size: {}", db.page_size);
            println!("number of tables: {}", db.get_page(1).entries().count());
        }
        Some(Cmd::Tables) => {
            for cell in db.get_page(1).entries() {
                print!("{} ", Schema::new(cell.payload).tbl_name);
            }
            println!()
        }
        None => {
            let query = query.context("no command or query provided")?;
            vm::handle_query(&db, &query)?;
        }
    }
    Ok(())
}
