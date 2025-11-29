use anyhow::Context;
use anyhow::Result;

use parser::ColumnDef;
use parser::CreateTable;
use parser::SelectColStmt;
use parser::SelectCols;
use parser::SqlType;
use parser::Value;
use parser::sql;

use crate::Database;
use crate::Entry;
use crate::Schema;
use crate::SerialType;
use crate::read_varint;

type ParsedEntry<'a> = Vec<(Value<'a>, &'a str)>;

pub fn handle_query(db: &Database, query: &str) -> Result<()> {
    let select = sql::select(query)?;
    let schema = get_tbl_schema(db, select.table)?;
    let ct = sql::create_table(&schema.sql).expect("corrupt table");
    // TODO: Implement where clause.
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
                print_row(selected, parse_entry(&ct, &e), Some);
            }
        }
        SelectColStmt::Count(_) => {
            println!("{}", db.get_page(schema.rootpage).entries().count())
        }
        SelectColStmt::Avg(col) => {
            dbg!(&col);
            let ct = sql::create_table(&schema.sql).expect("corrupt create table statement");

            let mut sum: f64 = 0.;
            let mut count: usize = 0;

            for entry in db.get_page(schema.rootpage).entries() {
                let cloned = entry.clone();
                let parsed_row = parse_entry(&ct, &cloned);

                for (val, c) in parsed_row {
                    if c.name == col {
                        count += 1;
                        match val {
                            Value::Int(i) => sum += i as f64,
                            Value::Float(i) => sum += i,
                            Value::Null => {}
                            Value::String(_) => panic!("Cannot add strings"),
                        }
                    }
                }
            }

            println!("{}", sum / count as f64);
        }
    }
    Ok(())
}

fn parse_entry<'a>(ct: &'a CreateTable<'a>, entry: &'a Entry) -> ParsedEntry<'a> {
    let (header_size, header_int_size) = read_varint(&mut entry.payload.as_slice());
    let mut header = &entry.payload[header_int_size as usize..header_size as usize];
    // HACK: Can we do this without allocation? By implementing a iterator on serial types.
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
                (Value::Int(entry.key), c.name)
            } else {
                (v, c.name)
            }
        })
        .collect()
}

fn print_row<'a>(
    selected: &[ColumnDef],
    pe: ParsedEntry<'a>,
    wrap: impl Fn(ParsedEntry<'a>) -> Option<ParsedEntry<'a>>,
) {
    let Some(result) = wrap(pe) else {
        return;
    };
    let mut iter = selected
        .iter()
        .flat_map(|s| result.iter().find_map(|(v, c)| (s.name == *c).then_some(v)));
    iter.next().inspect(|v| print!("{v}"));
    for v in iter {
        print!("|{v}")
    }
    println!()
}

fn get_tbl_schema(db: &Database, tbl_name: &str) -> Result<Schema> {
    db.get_page(1)
        .entries()
        .find_map(|e| {
            let s = Schema::new(e.payload);
            if s.tbl_name == tbl_name { Some(s) } else { None }
        })
        .with_context(|| format!("no such table: {tbl_name}"))
}
