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
use btree::Entry;
use cli::Args;
use cli::Cmd;
use parser::ColumnDef;
use parser::CreateTable;
use parser::SelectColStmt;
use parser::SelectCols;
use parser::SqlType;
use parser::Value;
use parser::sql;
use record::Schema;
use record::SerialType;
use varint::read_varint;

fn get_tbl_schema(db: &Database, table_name: &str) -> Result<Schema> {
    db.get_page(1)
        .entries()
        .find_map(|e| {
            let s = Schema::new(e.payload);
            if s.tbl_name == table_name { Some(s) } else { None }
        })
        .context(format!("no such table: {}", table_name))
}

fn parse_entry<'a>(ct: &'a CreateTable<'a>, entry: &'a Entry) -> Vec<(Value<'a>, &'a ColumnDef<'a>)> {
    let (header_size, header_int_size) = read_varint(&mut entry.payload.as_slice());
    let mut header = &entry.payload[header_int_size as usize..header_size as usize];
    let mut sts = vec![];
    while !header.is_empty() {
        let st = SerialType::from(read_varint(&mut header).0 as u64);
        sts.push(st);
    }
    let record_payload = &entry.payload[header_size as usize..];
    let values = SerialType::parse_payload(&sts, record_payload);
    values
        .zip(&ct.columns)
        .map(|(v, c)| {
            if c.primary_key && c.sql_type == SqlType::Integer {
                (Value::Int(entry.key), c)
            } else {
                (v, c)
            }
        })
        .collect()
}

fn print_row<'a>(selected: &[ColumnDef], vals: Vec<(Value<'a>, &'a ColumnDef<'a>)>) {
    let mut iter = selected
        .iter()
        .flat_map(|s| vals.iter().find_map(|(v, c)| (s.name == c.name).then_some(v)));
    iter.next().inspect(|v| print!("{v}"));
    for v in iter {
        print!("|{v}")
    }
    println!()
}

fn handle_select_query(db: &Database, query: &str) -> Result<()> {
    let select = sql::select(query)?;
    let schema = get_tbl_schema(db, select.table)?;
    let ct = sql::create_table(&schema.sql).expect("corrupt table");
    match select.columns {
        SelectColStmt::List(list) => {
            let selected = match list {
                SelectCols::List(cols) => &cols
                    .iter()
                    .flat_map(|col| ct.columns.iter().copied().find(|c| c.name == *col))
                    .collect(),
                SelectCols::All => &ct.columns,
            };
            for e in db.get_page(schema.rootpage).entries() {
                print_row(selected, parse_entry(&ct, &e));
            }
        }
        SelectColStmt::Count(_) => {
            println!("{}", db.get_page(schema.rootpage).entries().count())
        }
    }
    Ok(())
}

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
            handle_select_query(&db, &query)?;
        }
    }
    Ok(())
}
