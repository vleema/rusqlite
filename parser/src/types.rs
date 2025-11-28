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
pub enum Operator {
    Eq,
    Neq,
    Leq,
    Geq,
    Less,
    Greater,
}

#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug, PartialEq, Eq)]
pub struct WhereExpression<'a> {
<<<<<<< HEAD
    pub v1: &'a str,
    pub operator: Operator,
    pub v2: &'a str,
=======
    column: &'a str,
    operator: &'a str, //ver se isso seria sqltype::operator ou alguma coisa operator
    value: SqlType::Integer
>>>>>>> 1e25443 (começo do where statement)
}

#[derive(Debug, PartialEq, Eq)]
pub struct SelectWhere<'a> {
    expressions: Vec<&'a str>,
    operator: Vec<&'a str>, //talvez fazer um enum de portas logicas seria uma boa?
}
