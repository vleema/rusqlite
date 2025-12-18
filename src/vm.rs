use anyhow::Context;
use anyhow::Result;

use anyhow::anyhow;
use parser::ColumnDef;
use parser::CreateTable;
use parser::SelectColStmt;
use parser::SelectCols;
use parser::SqlType;
use parser::Value;
use parser::WhereExpr;
use parser::sql;

use crate::Database;
use crate::Entry;
use crate::Schema;
use crate::SerialType;
use crate::read_varint;

pub fn handle_query(db: &Database, query: &str) -> Result<()> {
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
                let pe = parse_entry(&ct, &e);
                if let Some(expr) = &select.expr
                    && !where_matches(expr, &pe)?
                {
                    continue;
                }
                print_row(selected, pe);
            }
        }
        SelectColStmt::Count(_) => {
            println!("{}", db.get_page(schema.rootpage).entries().count())
        }
        SelectColStmt::Avg(col) => {
            let ct = sql::create_table(&schema.sql).expect("corrupt create table statement");

            let mut sum: f64 = 0.;
            let mut count: usize = 0;

            for entry in db.get_page(schema.rootpage).entries() {
                let cloned = entry.clone();
                let parsed_row = parse_entry(&ct, &cloned);

                for (c, val) in parsed_row {
                    if c == col {
                        count += 1;
                        match val {
                            Value::Int(i) => sum += i as f64,
                            Value::Float(i) => sum += i,
                            Value::Null => {}
                            Value::String(s) => sum += s.parse::<f64>().unwrap(),
                        }
                    }
                }
            }

            println!("{}", sum / count as f64);
        }
    }
    Ok(())
}

type ParsedEntry<'a> = Vec<(&'a str, Value<'a>)>;

fn where_matches<'a>(we: &WhereExpr<'a>, pe: &'a ParsedEntry<'a>) -> Result<bool> {
    use WhereExpr::*;
    let get_col_value = |colname: &'a str, pe: &'a ParsedEntry<'a>| {
        pe.iter()
            .find_map(|(name, val)| (*name == colname).then_some(val))
            .ok_or(anyhow!("invalid column name '{colname}' in where expression"))
    };

    Ok(match we {
        Neq(c, v) => get_col_value(c, pe)? != v,
        Eq(c, v) => get_col_value(c, pe)? == v,
        Leq(c, v) => get_col_value(c, pe)? <= v,
        Geq(c, v) => get_col_value(c, pe)? >= v,
        Le(c, v) => get_col_value(c, pe)? < v,
        Ge(c, v) => get_col_value(c, pe)? > v,
        And(l, r) => where_matches(l, pe)? && where_matches(r, pe)?,
        Or(l, r) => where_matches(l, pe)? || where_matches(r, pe)?,
    })
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
                (c.name, Value::Int(entry.key))
            } else {
                (c.name, v)
            }
        })
        .collect()
}

fn print_row<'a>(selected: &[ColumnDef], pe: ParsedEntry<'a>) {
    let mut iter = selected
        .iter()
        .flat_map(|s| pe.iter().find_map(|(c, v)| (s.name == *c).then_some(v)));
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
