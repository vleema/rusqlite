use parser::{SqlType, Value};

use crate::btree::PageNumber;
use crate::varint::read_varint;

#[derive(Debug, Clone, Copy)]
pub enum SerialType {
    Null,
    Int8,
    Int16,
    Int24,
    Int32,
    Int48,
    Int64,
    Float,
    Zero,
    One,
    Internal,
    Blob { size: u64 },
    Text { size: u64 },
}

impl SerialType {
    pub fn parse_payload<'a>(sts: &'a [SerialType], payload: &'a [u8]) -> impl Iterator<Item = Value<'a>> {
        use SerialType as T;
        use Value as V;
        let mut cursor = payload;
        sts.iter().filter_map(move |st| {
            Some(match st {
                T::Null => V::Null,
                T::Int8 => {
                    let val = i64::from_be_bytes([0, 0, 0, 0, 0, 0, 0, cursor[0]]);
                    cursor = &cursor[1..];
                    V::Int(val)
                }
                T::Int16 => {
                    let val = i64::from_be_bytes([0, 0, 0, 0, 0, 0, cursor[0], cursor[1]]);
                    cursor = &cursor[2..];
                    V::Int(val)
                }
                T::Int24 => {
                    let val = i64::from_be_bytes([0, 0, 0, 0, 0, cursor[0], cursor[1], cursor[2]]);
                    cursor = &cursor[3..];
                    V::Int(val)
                }
                T::Int32 => {
                    let val = i64::from_be_bytes([0, 0, 0, 0, cursor[0], cursor[1], cursor[2], cursor[3]]);
                    cursor = &cursor[4..];
                    V::Int(val)
                }
                T::Int48 => {
                    let val =
                        i64::from_be_bytes([0, 0, cursor[0], cursor[1], cursor[2], cursor[3], cursor[4], cursor[5]]);
                    cursor = &cursor[6..];
                    V::Int(val)
                }
                T::Int64 => {
                    let val = i64::from_be_bytes([
                        cursor[0], cursor[1], cursor[2], cursor[3], cursor[4], cursor[5], cursor[6], cursor[7],
                    ]);
                    cursor = &cursor[8..];
                    V::Int(val)
                }
                T::Float => {
                    let val = f64::from_be_bytes([
                        cursor[0], cursor[1], cursor[2], cursor[3], cursor[4], cursor[5], cursor[6], cursor[7],
                    ]);
                    cursor = &cursor[8..];
                    V::Float(val)
                }
                T::Zero => V::Int(0),
                T::One => V::Int(1),
                T::Internal => None?,
                T::Blob { .. } => None?,
                T::Text { size } => V::String(next_utf8(&mut cursor, *size as usize)),
            })
        })
    }
}

impl From<u64> for SerialType {
    fn from(value: u64) -> Self {
        use SerialType::*;
        match value {
            0 => Null,
            1 => Int8,
            2 => Int16,
            3 => Int24,
            4 => Int32,
            5 => Int48,
            6 => Int64,
            7 => Float,
            8 => Zero,
            9 => One,
            10 | 11 => Internal,
            n if n % 2 == 0 => Blob { size: (n - 12) / 2 },
            n => Text { size: (n - 13) / 2 },
        }
    }
}

impl From<SerialType> for u64 {
    fn from(value: SerialType) -> Self {
        use SerialType::*;
        match value {
            Null => 0,
            Int8 => 1,
            Int16 => 2,
            Int24 => 3,
            Int32 => 4,
            Int48 => 5,
            Int64 => 6,
            Float => 7,
            Zero => 8,
            One => 9,
            Internal => 10,
            Blob { size } => (size * 2) + 12,
            Text { size } => (size * 2) + 13,
        }
    }
}

impl From<SerialType> for Option<SqlType> {
    fn from(value: SerialType) -> Self {
        use SerialType as St;
        use SqlType as T;
        Some(match value {
            St::Null => None?,
            St::Int8 => T::Integer,
            St::Int16 => T::Integer,
            St::Int24 => T::Integer,
            St::Int32 => T::Integer,
            St::Int48 => T::Integer,
            St::Int64 => T::Integer,
            St::Float => T::Integer,
            St::Zero => T::Numeric,
            St::One => T::Numeric,
            St::Internal => None?,
            St::Blob { .. } => T::Blob,
            St::Text { .. } => T::Text,
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Schema {
    pub ty: String,
    pub name: String,
    pub tbl_name: String,
    pub rootpage: PageNumber,
    pub sql: String,
}

impl Schema {
    pub fn new(payload: Vec<u8>) -> Self {
        use SerialType::*;

        let mut cursor = payload.as_slice();

        read_varint(&mut cursor); // header size
        let Text { size: ty_size } = SerialType::from(read_varint(&mut cursor).0 as u64) else {
            panic!("invalid serial type for schema type")
        };
        let Text { size: name_size } = SerialType::from(read_varint(&mut cursor).0 as u64) else {
            panic!("invalid serial type for schema name")
        };
        let Text { size: tbl_size } = SerialType::from(read_varint(&mut cursor).0 as u64) else {
            panic!("invalid serial type for schema tbl_name")
        };
        match read_varint(&mut cursor).0 as u64 {
            1..=6 | 8 | 9 => {}
            _ => panic!("invalid serial type for schema rootpage"),
        };
        let Text { size: sql_size } = SerialType::from(read_varint(&mut cursor).0 as u64) else {
            panic!("invalid serial type for schema sql")
        };
        let ty = next_utf8(&mut cursor, ty_size as usize).to_owned();
        let name = next_utf8(&mut cursor, name_size as usize).to_owned();
        let tbl_name = next_utf8(&mut cursor, tbl_size as usize).to_owned();
        // If the rootpage is negative or doesn't fit... 💥
        let rootpage = read_varint(&mut cursor).0 as u32;
        let sql = next_utf8(&mut cursor, sql_size as usize).to_owned();

        Self {
            ty,
            name,
            tbl_name,
            rootpage,
            sql,
        }
    }
}

fn next_utf8<'a>(v: &mut &'a [u8], size: usize) -> &'a str {
    assert!(size <= v.len());
    let buf = &v[..size];
    *v = &v[size..];
    std::str::from_utf8(buf).expect("invalid utf8 string")
}
