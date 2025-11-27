#[derive(Debug, PartialEq)]
pub enum Value<'a> {
    String(&'a str),
    Bool(bool),
    Float(f64),
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

#[derive(Debug, PartialEq, Eq)]
pub struct Select<'a> {
    pub columns: SelectColStmt<'a>,
    pub table: &'a str,
    pub expr: Option<()>,
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
