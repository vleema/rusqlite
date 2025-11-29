use std::fs::File;

use anyhow::Context;
use anyhow::Ok;
use anyhow::Result;
use anyhow::bail;
use clap::Parser;

mod btree;
mod cli;
mod record;
mod varint;

use btree::Database;
use cli::Args;
use cli::Cmd;
use parser::SelectColStmt;
use parser::SelectCols;
use parser::SqlType;
use parser::sql;
use record::Schema;

use crate::record::SerialType;
use crate::varint::read_varint;

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
            // dbg!(db.get_page(5));
            let query = query.context("no command or query provided")?;
            let select = sql::select(&query)?;
            let Some(schema) = db.get_page(1).entries().find_map(|e| {
                let s = Schema::new(e.payload);
                if s.tbl_name == select.table { Some(s) } else { None }
            }) else {
                bail!("table does not exist")
            };
            let SelectColStmt::List(SelectCols::List(list)) = select.columns else {
                todo!("impl select count")
            };
            let ct = sql::create_table(&schema.sql).expect("corrupt create table statement");
            let mut ct_cols = ct.columns.iter().map(|c| (c, false)).collect::<Vec<_>>();
            for col in list {
                let Some((_, enabled)) = ct_cols.iter_mut().find(|(c, _)| c.name == col) else {
                    bail!("invalid column name: {col}")
                };
                *enabled = true;
            }
            for e in db.get_page(schema.rootpage).entries() {
                let (header_size, header_int_size) = read_varint(&mut e.payload.as_slice());
                let mut header = &e.payload[header_int_size as usize..header_size as usize];
                let mut sts = vec![];
                while !header.is_empty() {
                    let st = SerialType::from(read_varint(&mut header).0 as u64);
                    sts.push(st);
                }
                for (v, c) in SerialType::parse_payload(&sts, &e.payload[header_size as usize..])
                    .zip(&ct_cols)
                    .filter_map(|(v, (c, e))| e.then_some((v, c)))
                {
                    if c.primary_key && c.sql_type == SqlType::Integer {
                        print!("{}|", e.rowid);
                        continue;
                    }
                    print!("{v}|");
                }
                println!()
            }
        }
    }

    Ok(())
}
