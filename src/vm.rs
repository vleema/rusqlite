use anyhow::Context;
use anyhow::Result;

use parser::ColumnDef;
use parser::CreateTable;
use parser::Op;
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

type ParsedEntry<'a> = Vec<(&'a str, Value<'a>)>;

fn match_sql_type_with_value(sql_t: &SqlType, v: &Value) -> bool {
    match v {
        Value::String(_) => *sql_t == SqlType::Text,
        Value::Float(_) => *sql_t == SqlType::Numeric || *sql_t == SqlType::Real,
        Value::Int(_) => *sql_t == SqlType::Integer,
        Value::Null => true,
    }
}

// TODO: maybe the op should be more generic
fn cmp<'a>(row: &ParsedEntry<'a>, col: &str, v: &Value, o: Op) -> bool {
    let Some((_, row_value)) = row.iter().find(|(c, _)| *c == col) else {
        return false;
    };

    match o {
        Op::Eq => row_value == v,
        Op::Neq => row_value != v,
        Op::Le => row_value < v,
        Op::Leq => row_value <= v,
        Op::Ge => row_value > v,
        Op::Geq => row_value >= v,
    }
}

fn criteria<'a>(expr: &Option<WhereExpr<'a>>, row: &ParsedEntry<'a>) -> bool {
    use WhereExpr::*;

    match expr {
        None => true, // select without where
        Some(e) => match &e {
            Eq(col, v) => cmp(row, col, v, Op::Eq),
            Neq(col, v) => cmp(row, col, v, Op::Neq),
            Le(col, v) => cmp(row, col, v, Op::Le),
            Ge(col, v) => cmp(row, col, v, Op::Ge),
            Leq(col, v) => cmp(row, col, v, Op::Leq),
            Geq(col, v) => cmp(row, col, v, Op::Geq),
            And(e1, e2) => criteria(&Some((**e1).clone()), row) && criteria(&Some((**e2).clone()), row),
            Or(e1, e2) => criteria(&Some((**e1).clone()), row) || criteria(&Some((**e2).clone()), row),
        },
    }
}

// verify if the column referenced exists, and if so, if the types column and value match
// example of behavior that should stop: `select name from users where idade = true`
fn valid(expr: &Option<WhereExpr>, table: &CreateTable) -> bool {
    use WhereExpr::*;

    match expr {
        None => true, // select without where
        Some(e) => {
            // select com where
            match &e {
                Neq(col_name, v)
                | Eq(col_name, v)
                | Leq(col_name, v)
                | Geq(col_name, v)
                | Le(col_name, v)
                | Ge(col_name, v) => {
                    match table.columns.iter().find(|x| &x.name == col_name) {
                        None => false, // collumn referenced on where dont exist
                        Some(&c) => match_sql_type_with_value(&c.sql_type, v),
                    }
                }
                And(e1, e2) | Or(e1, e2) => valid(&Some((**e1).clone()), table) && valid(&Some((**e2).clone()), table),
            }
        }
    }
}

pub fn handle_query(db: &Database, query: &str) -> Result<()> {
    let select = sql::select(query)?;
    let schema = get_tbl_schema(db, select.table)?;
    let ct = sql::create_table(&schema.sql).expect("corrupt table");

    if !valid(&select.expr, &ct) {
        return Err(anyhow::anyhow!("Invalid where expression"));
    }

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
                // Passar função que vocês implementaram aqui ↓
                print_row(selected, parse_entry(&ct, &e), |row| criteria(&select.expr, row));
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

fn print_row<'a>(selected: &[ColumnDef], pe: ParsedEntry<'a>, p: impl FnOnce(&ParsedEntry<'a>) -> bool) {
    if !p(&pe) {
        return;
    };
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
