mod types;

pub use types::*;

peg::parser! {
    pub grammar sql() for str {
        rule _ = quiet! { [' '|'\t'|'\n'] }

        rule integer() -> i64
            = quiet! { n:$("-"? ['0'..='9']+) {? n.parse().or(Err("i64")) } }

        rule float() -> f64
            = quiet! { n:$("-"? ['0'..='9']+ "." ['0'..='9']*) {? n.parse().or(Err("f64")) } }

        rule boolean() -> bool
            = quiet! { ("true"/"1") { true } / ("false"/"0") { false } }


        rule identifier() -> &'input str
            = l:$(['a'..='z'|'A'..='Z'|'_']['a'..='z'|'A'..='Z'|'_'|'0'..='9']*) { l }

        // How to implement ignore case?
        pub rule ty() -> SqlType
            = ("INTEGER"/"INT")            { SqlType::Integer }
            / ("VARCHAR"/"TEXT")           { SqlType::Text }
            / ("DOUBLE"/"REAL")            { SqlType::Real }
            / ("NUMERIC"/"BOOLEAN"/"DATE") { SqlType::Numeric }
            / "BLOB"                       { SqlType::Blob }

        pub rule value() -> Value<'input>
            = "'" s:identifier() "'"  { Value::String(s) }
            / b:boolean()             { Value::Bool(b) }
            / f:float()               { Value::Float(f) }
            / n:integer()             { Value::Int(n) }

        rule columns() -> SelectCols<'input>
            = l:(identifier() ++ ("," _*)) { SelectCols::List(l) }
            / "*"                          { SelectCols::All }

        rule constraint() -> &'input str
            = !"PRIMARY" s:identifier() { s }

        pub rule select_column_stmt() ->  SelectColStmt<'input>
            = "COUNT" _* "(" _* c:columns() _* ")" { SelectColStmt::Count(c) }
            / c:columns()                          { SelectColStmt::List(c) }

        pub rule select() -> Select<'input>
            = "SELECT" _+ c:select_column_stmt() _+ "FROM" _+ t:identifier()
                { Select { columns: c, table: t, expr: None }}

        pub rule column_def() -> ColumnDef<'input>
            = n:identifier() _+ t:ty() _+ (constraint() _+)* "PRIMARY" _+ "KEY" (_+ constraint())*
                { ColumnDef { sql_type: t, name: n, primary_key: true } }
            / n:identifier() _+ t:ty() (_+ constraint())*
                { ColumnDef { sql_type: t, name: n, primary_key: false } }
            / n:identifier() { ColumnDef { sql_type: SqlType::Text, name: n, primary_key: false }}

        pub rule create_table() -> CreateTable<'input>
            = "CREATE" _+ "TABLE" _+ t:identifier() _+ "(" _* c:(column_def() ++ ("," _*)) _* ")" {
                // Select first colunm as primary key if no one was specified.
                let mut primary_key = 0;
                for (i, c) in c[1..].iter().enumerate() {
                    if c.primary_key  {
                        primary_key = i;
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
            Ok(SelectColStmt::Count(SelectCols::List(vec![
                "id",
                "name",
                "created_at"
            ])))
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
            sql::create_table(
                "CREATE TABLE users ( id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT )"
            ),
            Ok(CreateTable {
                table_name: "users",
                columns: vec![
                    ColumnDef {
                        sql_type: SqlType::Integer,
                        name: "id",
                        primary_key: true
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
    }
}
