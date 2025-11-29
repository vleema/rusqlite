mod types;

pub use types::*;

peg::parser! {
    pub grammar sql() for str {
        rule _ = quiet! { [' '|'\t'|'\n'] }

        rule integer() -> i64
            = quiet! { ("true") { 1 } / ("false") { 0 } }
            / quiet! { n:$("-"? ['0'..='9']+) {? n.parse().or(Err("i64")) } }

        rule float() -> f64
            = quiet! { n:$("-"? ['0'..='9']+ "." ['0'..='9']*) {? n.parse().or(Err("f64")) } }


        rule i(literal: &'static str)
            = input:$([_]*<{literal.len()}>)
                {? if input.eq_ignore_ascii_case(literal) { Ok(()) } else { Err(literal) }}

        rule identifier() -> &'input str
            = l:$(['a'..='z'|'A'..='Z'|'_']['a'..='z'|'A'..='Z'|'_'|'0'..='9']*) { l }

        pub rule ty() -> SqlType
            = (i("integer")/i("int"))               { SqlType::Integer }
            / (i("varchar")/i("text"))              { SqlType::Text }
            / (i("double")/i("real"))               { SqlType::Real }
            / (i("numeric")/i("boolean")/i("date")) { SqlType::Numeric }
            / i("blob")                             { SqlType::Blob }

        pub rule value() -> Value<'input>
            = "'" s:identifier() "'"  { Value::String(s) }
            / f:float()               { Value::Float(f) }
            / n:integer()             { Value::Int(n) }

        rule columns() -> SelectCols<'input>
            = l:(identifier() ++ ("," _*)) { SelectCols::List(l) }
            / "*"                          { SelectCols::All }

        rule constraint() -> &'input str
            = !i("primary") s:identifier() { s }

        pub rule select_column_stmt() ->  SelectColStmt<'input>
            = i("count") _* "(" _* c:columns() _* ")" { SelectColStmt::Count(c) }
            / i("avg") _* "(" _* s:identifier() _* ")"{ SelectColStmt::Avg(s)   }
            / c:columns()                             { SelectColStmt::List(c)  }

        pub rule select() -> Select<'input>
            = i("select") _+ c:select_column_stmt() _+ i("from") _+ t:identifier()
                { Select { columns: c, table: t, expr: None } }

        pub rule column_def() -> ColumnDef<'input>
            = n:identifier() _+ t:ty() _+ (constraint() _+)* i("primary") _+ i("key") (_+ constraint())*
                { ColumnDef { sql_type: t, name: n, primary_key: true } }
            / n:identifier() _+ t:ty() (_+ constraint())*
                { ColumnDef { sql_type: t, name: n, primary_key: false } }
            / n:identifier() { ColumnDef { sql_type: SqlType::Text, name: n, primary_key: false }}

        pub rule create_table() -> CreateTable<'input>
            = i("create") _+ i("table") _+ t:identifier() _* "(" _* c:(column_def() ++ ("," _*)) _* ")" {
                // Select first column as primary key if no one was specified.
                let mut primary_key = 0;
                for (i, c) in c[1..].iter().enumerate() {
                    if c.primary_key {
                        primary_key = i + 1;
                    }
                }
                CreateTable {
                    table_name: t,
                    columns: c,
                    primary_key,
                }
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ty() {
        assert_eq!(sql::ty("VARCHAR"), Ok(SqlType::Text));
    }

    #[test]
    fn value() {
        assert_eq!(sql::value("'name'"), Ok(Value::String("name")));
    }

    #[test]
    fn select_column_stmt() {
        assert_eq!(
            sql::select_column_stmt("COUNT( id,name,   created_at )"),
            Ok(SelectColStmt::Count(SelectCols::List(vec!["id", "name", "created_at"])))
        );
        assert_eq!(
            sql::select_column_stmt("COUNT(*)"),
            Ok(SelectColStmt::Count(SelectCols::All))
        );
    }

    #[test]
    fn select() {
        assert_eq!(
            sql::select("SELECT name FROM users"),
            Ok(Select {
                columns: SelectColStmt::List(SelectCols::List(vec!["name"])),
                table: "users",
                expr: None
            })
        );
        assert_eq!(
            sql::select("SELECT id,   name, \tcreated_at FROM users"),
            Ok(Select {
                columns: SelectColStmt::List(SelectCols::List(vec!["id", "name", "created_at"])),
                table: "users",
                expr: None
            })
        );
    }

    #[test]
    fn create_table() {
        assert_eq!(
            sql::create_table("CREATE TABLE users ( id INTEGER )"),
            Ok(CreateTable {
                table_name: "users",
                columns: vec![ColumnDef {
                    sql_type: SqlType::Integer,
                    name: "id",
                    primary_key: false
                }],
                primary_key: 0
            })
        );
        assert_eq!(
            sql::create_table("create table users(id,name)"),
            Ok(CreateTable {
                table_name: "users",
                columns: vec![
                    ColumnDef {
                        sql_type: SqlType::Text,
                        name: "id",
                        primary_key: false
                    },
                    ColumnDef {
                        sql_type: SqlType::Text,
                        name: "name",
                        primary_key: false
                    }
                ],
                primary_key: 0,
            })
        );
        assert_eq!(
            sql::create_table(
                "create table produto (id integer primary key autoincrement, nome text not null, preco text not null)"
            ),
            Ok(CreateTable {
                table_name: "produto",
                columns: vec![
                    ColumnDef {
                        sql_type: SqlType::Integer,
                        name: "id",
                        primary_key: true
                    },
                    ColumnDef {
                        sql_type: SqlType::Text,
                        name: "nome",
                        primary_key: false,
                    },
                    ColumnDef {
                        sql_type: SqlType::Text,
                        name: "preco",
                        primary_key: false
                    }
                ],
                primary_key: 0
            })
        )
    }
}
