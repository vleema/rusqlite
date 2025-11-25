use crate::database::PageNumber;
use crate::varint::read_varint;

#[derive(Debug)]
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

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct SchemaCell<'a> {
    pub ty: &'a str,
    pub name: &'a str,
    pub tbl_name: &'a str,
    pub rootpage: PageNumber,
    pub sql: &'a str,
}

impl<'a> SchemaCell<'a> {
    pub fn new(mut payload: &'a [u8]) -> Self {
        use SerialType::*;

        let (header_size, header_varint_size) = read_varint(&mut payload);
        let header_size = (header_size - header_varint_size as i64) as usize;

        let mut cursor = &payload[..header_size];
        let Text { size: ty_size } = SerialType::from(read_varint(&mut cursor).0 as u64) else {
            panic!("invalid serial type for type")
        };
        let Text { size: name_size } = SerialType::from(read_varint(&mut cursor).0 as u64) else {
            panic!("invalid serial type for name")
        };
        let Text { size: tbl_size } = SerialType::from(read_varint(&mut cursor).0 as u64) else {
            panic!("invalid serial type for tbl_name")
        };
        match read_varint(&mut cursor).0 as u64 {
            1..=6 | 8 | 9 => {}
            _ => panic!("invalid serial type for rootpage"),
        };
        let Text { size: sql_size } = SerialType::from(read_varint(&mut cursor).0 as u64) else {
            panic!("invalid serial type for sql")
        };

        cursor = &payload[header_size..];
        let ty = next_utf8(&mut cursor, ty_size as usize);
        let name = next_utf8(&mut cursor, name_size as usize);
        let tbl_name = next_utf8(&mut cursor, tbl_size as usize);
        // If the rootpage is negative or doesn't fit... 💥
        let rootpage = read_varint(&mut cursor).0 as u32;
        let sql = next_utf8(&mut cursor, sql_size as usize);

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
