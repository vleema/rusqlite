use std::fmt::Display;

#[derive(Debug, PartialEq, Clone)]
pub enum Value<'a> {
    String(&'a str),
    // Bool(bool),
    Float(f64),
    Int(i64),
    Null,
}

impl Display for Value<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(v) => write!(f, "{v}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::Int(v) => write!(f, "{v}"),
            // Self::Bool(v) => write!(f, "{v}"),
            Self::Null => write!(f, "NULL"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
    Avg(&'a str),
}

#[derive(Debug, PartialEq)]
pub struct Select<'a> {
    pub columns: SelectColStmt<'a>,
    pub table: &'a str,
    pub expr: Option<WhereExpr<'a>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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

#[derive(Debug, PartialEq, Clone)]
pub enum WhereExpr<'a> {
    Neq(&'a str, Value<'a>),
    Eq(&'a str, Value<'a>),
    Leq(&'a str, Value<'a>),
    Geq(&'a str, Value<'a>),
    Le(&'a str, Value<'a>),
    Ge(&'a str, Value<'a>),
    And(Box<WhereExpr<'a>>, Box<WhereExpr<'a>>),
    Or(Box<WhereExpr<'a>>, Box<WhereExpr<'a>>),
}
