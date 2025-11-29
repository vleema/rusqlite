#[derive(Debug, PartialEq)]
pub enum Value<'a> {
    String(&'a str),
    Bool(bool),
    Float(f64), //VER DEPOIS ESSE PROBLEMA DO EQL COM O FLOAT, SE EU N PASSAR NADA, OU UM NAN, OQ ACONTECE?
    Int(i64),
}

#[derive(Debug, PartialEq, Eq)]
pub enum SqlType {
    Integer,
    Text,
    Real,
    Numeric,
    Blob,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SelectCols<'a> {
    List(Vec<&'a str>),
    All,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SelectColStmt<'a> {
    List(SelectCols<'a>),
    Count(SelectCols<'a>),
}

#[derive(Debug, PartialEq)]
pub struct Select<'a> {
    pub columns: SelectColStmt<'a>,
    pub table: &'a str,
    pub expr: Option<WhereExpression<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ColumnDef<'a> {
    pub sql_type: SqlType,
    pub name: &'a str,
    pub primary_key: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CreateTable<'a> {
    pub table_name: &'a str,
    pub columns: Vec<ColumnDef<'a>>,
    pub primary_key: usize,
}

#[derive(Debug, PartialEq)]
pub enum WhereExpression<'a> {
    Neq(&'a str, Value<'a>),
    Eq(&'a str, Value<'a>),
    Leq(&'a str, Value<'a>),
    Geq(&'a str, Value<'a>),
    Less(&'a str, Value<'a>),
    Greater(&'a str, Value<'a>),
    AND(Box<WhereExpression<'a>>, Box<WhereExpression<'a>>),
    OR(Box<WhereExpression<'a>>, Box<WhereExpression<'a>>),
}
