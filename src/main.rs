use std::fs::File;

use anyhow::Context;
use anyhow::Ok;
use anyhow::Result;
use clap::Parser;

mod btree;
mod cli;
mod record;
mod varint;

use btree::Database;
use cli::Args;
use cli::Cmd;
use parser::sql;
use record::Schema;

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
            let select = sql::select(&query)?;
            for schema_entry in db.get_page(1).entries() {
                let schema = Schema::new(schema_entry.payload);
                if schema.tbl_name == select.table {
                    println!("{}", db.get_page(schema.rootpage).entries().count());
                }
            }
        }
    }

    Ok(())
}
